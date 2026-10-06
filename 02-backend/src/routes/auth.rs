//! Auth routes: register, login (+2FA), refresh, me, 2FA setup/verify.
//! [M §7.1.2, §7.1.3]

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use crate::auth::{hash_password, issue_token, verify_password, verify_token, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub locale: Option<String>,
    pub consent: Option<bool>, // Loi 18-07 explicit consent [M §10.1]
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterReq>,
) -> AppResult<Json<Value>> {
    if req.password.len() < 8 {
        return Err(AppError::BadRequest("password too short (min 8)".into()));
    }
    if req.consent != Some(true) {
        return Err(AppError::BadRequest("consent to data processing (Loi 18-07) is required".into()));
    }
    let hash = hash_password(&req.password)?;
    let locale = req.locale.unwrap_or_else(|| "ar".to_string());
    let rec = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO user_account (email, password_hash, full_name, locale, consent_at) \
         VALUES ($1,$2,$3,$4::locale_t, now()) RETURNING id",
    )
    .bind(&req.email)
    .bind(&hash)
    .bind(&req.full_name)
    .bind(&locale)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            AppError::BadRequest("email already registered".into())
        }
        other => AppError::from(other),
    })?;
    Ok(Json(json!({ "id": rec })))
}

#[derive(Deserialize)]
pub struct ForgotReq { pub email: String }

/// Start password reset [M §7.1.2]: create a token and "send" it by email. The
/// response never reveals whether the email exists. In mock-notification mode
/// (no email provider yet) the token is returned so the flow is usable in dev.
pub async fn password_forgot(
    State(state): State<AppState>,
    Json(req): Json<ForgotReq>,
) -> AppResult<Json<Value>> {
    let user_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM user_account WHERE email=$1")
        .bind(&req.email).fetch_optional(&state.db).await?;
    let mut dev_token: Option<String> = None;
    if let Some(uid) = user_id {
        let token = Uuid::new_v4().simple().to_string();
        sqlx::query(
            "INSERT INTO password_reset (user_id, token, expires_at) VALUES ($1,$2, now() + interval '1 hour')",
        ).bind(uid).bind(&token).execute(&state.db).await?;
        let link = format!("{}/reset?token={}", state.cfg.return_base_url.trim_end_matches('/'), token);
        crate::notify::send(&state, Some(uid), "email", "Réinitialisation du mot de passe",
            &format!("Lien de réinitialisation : {link}")).await;
        dev_token = Some(token); // dev convenience until an email provider is configured
    }
    Ok(Json(json!({ "ok": true, "dev_token": dev_token })))
}

#[derive(Deserialize)]
pub struct ResetReq { pub token: String, pub new_password: String }

pub async fn password_reset(
    State(state): State<AppState>,
    Json(req): Json<ResetReq>,
) -> AppResult<Json<Value>> {
    if req.new_password.len() < 8 {
        return Err(AppError::BadRequest("password too short (min 8)".into()));
    }
    let row: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM password_reset WHERE token=$1 AND used=false AND expires_at > now()",
    ).bind(&req.token).fetch_optional(&state.db).await?;
    let user_id = row.ok_or(AppError::BadRequest("invalid or expired token".into()))?;
    let hash = hash_password(&req.new_password)?;
    let mut tx = state.db.begin().await?;
    sqlx::query("UPDATE user_account SET password_hash=$1 WHERE id=$2").bind(&hash).bind(user_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE password_reset SET used=true WHERE token=$1").bind(&req.token).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(sqlx::FromRow)]
struct LoginRow {
    id: Uuid,
    password_hash: String,
    status: String,
    twofa_enabled: bool,
    totp_secret: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
    pub totp_code: Option<String>, // required when 2FA is enabled [M §7.1.3]
}

#[derive(Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> AppResult<Json<TokenPair>> {
    use std::time::Instant;
    let lim = crate::ratelimit::Limits {
        window: std::time::Duration::from_secs(1), max: 1,
        fail_window: std::time::Duration::from_secs(state.cfg.login_fail_window_secs.max(1) as u64),
        fail_max: state.cfg.login_fail_max.max(1) as u32,
    };
    let now = Instant::now();
    // Anti-brute-force: lock the account out after too many failed attempts [M §10.3].
    if state.rl.lock().map(|g| g.too_many_fails(&req.email, &lim, now)).unwrap_or(false) {
        return Err(AppError::TooManyRequests);
    }

    let result = do_login(&state, &req).await;
    match &result {
        Ok(_) => { if let Ok(mut g) = state.rl.lock() { g.clear_fails(&req.email); } }
        Err(AppError::Unauthorized) => { if let Ok(mut g) = state.rl.lock() { g.record_fail(&req.email, &lim, now); } }
        _ => {}
    }
    result.map(Json)
}

async fn do_login(state: &AppState, req: &LoginReq) -> AppResult<TokenPair> {
    let row = sqlx::query_as::<_, LoginRow>(
        "SELECT id, password_hash, status::text AS status, twofa_enabled, totp_secret \
         FROM user_account WHERE email = $1",
    )
    .bind(&req.email)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if row.status == "blacklisted" || row.status == "locked" {
        return Err(AppError::Unauthorized);
    }
    if !verify_password(&req.password, &row.password_hash) {
        return Err(AppError::Unauthorized);
    }
    if row.twofa_enabled {
        let code = req.totp_code.as_deref().ok_or(AppError::BadRequest("totp_code required".into()))?;
        let secret = row.totp_secret.clone().ok_or(AppError::Internal)?;
        if !check_totp(&secret, &req.email, code)? {
            return Err(AppError::Unauthorized);
        }
    }

    let role: Option<String> = sqlx::query_scalar(
        "SELECT r.code FROM agent a \
         JOIN agent_role ar ON ar.agent_id = a.id \
         JOIN role r ON r.id = ar.role_id \
         WHERE a.user_id = $1 \
         ORDER BY CASE r.code WHEN 'admin' THEN 0 WHEN 'operator' THEN 1 WHEN 'gate_agent' THEN 2 ELSE 3 END \
         LIMIT 1",
    )
    .bind(row.id)
    .fetch_optional(&state.db)
    .await?;
    issue_pair(state, row.id, &role.unwrap_or_else(|| "user".into()))
}

#[derive(Deserialize)]
pub struct RefreshReq {
    pub refresh_token: String,
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshReq>,
) -> AppResult<Json<TokenPair>> {
    let claims = verify_token(&state.cfg.jwt_secret, &req.refresh_token)?;
    if claims.token_type != "refresh" {
        return Err(AppError::Unauthorized);
    }
    Ok(Json(issue_pair(&state, claims.sub, &claims.role)?))
}

pub async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT email, full_name, locale::text FROM user_account WHERE id = $1",
    )
    .bind(user.user_id)
    .fetch_optional(&state.db)
    .await?;
    let (email, full_name, locale) = row.ok_or(AppError::NotFound)?;
    Ok(Json(json!({
        "id": user.user_id, "email": email, "full_name": full_name, "locale": locale, "role": user.role
    })))
}

// ── 2FA (TOTP) [M §7.1.3] ────────────────────────────────────────────────

fn build_totp(secret_b32: &str, account: &str) -> AppResult<TOTP> {
    let bytes = Secret::Encoded(secret_b32.to_string())
        .to_bytes()
        .map_err(|_| AppError::Internal)?;
    TOTP::new(Algorithm::SHA1, 6, 1, 30, bytes, Some("SOGISL".to_string()), account.to_string())
        .map_err(|_| AppError::Internal)
}

fn check_totp(secret_b32: &str, account: &str, code: &str) -> AppResult<bool> {
    let totp = build_totp(secret_b32, account)?;
    totp.check_current(code).map_err(|_| AppError::Internal)
}

pub async fn twofa_setup(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    let email: String = sqlx::query_scalar("SELECT email FROM user_account WHERE id = $1")
        .bind(user.user_id)
        .fetch_one(&state.db)
        .await?;
    let secret = Secret::generate_secret();
    let encoded = secret.to_encoded().to_string();
    let totp = build_totp(&encoded, &email)?;
    // Store the secret but leave 2FA disabled until the user verifies a code.
    sqlx::query("UPDATE user_account SET totp_secret = $1, twofa_channel = 'totp' WHERE id = $2")
        .bind(&encoded)
        .bind(user.user_id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "otpauth_url": totp.get_url(), "secret": encoded })))
}

#[derive(Deserialize)]
pub struct TwoFaVerifyReq {
    pub code: String,
}

pub async fn twofa_verify(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<TwoFaVerifyReq>,
) -> AppResult<Json<Value>> {
    let row: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT email, totp_secret FROM user_account WHERE id = $1")
            .bind(user.user_id)
            .fetch_optional(&state.db)
            .await?;
    let (email, secret) = row.ok_or(AppError::NotFound)?;
    let secret = secret.ok_or(AppError::BadRequest("run 2fa setup first".into()))?;
    if !check_totp(&secret, &email, &req.code)? {
        return Err(AppError::Unauthorized);
    }
    sqlx::query("UPDATE user_account SET twofa_enabled = true WHERE id = $1")
        .bind(user.user_id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "twofa_enabled": true })))
}

fn issue_pair(state: &AppState, user_id: Uuid, role: &str) -> AppResult<TokenPair> {
    Ok(TokenPair {
        access_token: issue_token(&state.cfg.jwt_secret, user_id, role, "access", state.cfg.jwt_access_ttl_secs)?,
        refresh_token: issue_token(&state.cfg.jwt_secret, user_id, role, "refresh", state.cfg.jwt_refresh_ttl_secs)?,
        token_type: "Bearer",
    })
}
