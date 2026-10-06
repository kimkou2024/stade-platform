//! User account management [M §7.1.2, §7.7, §10.1]: rectify profile, right to
//! deletion, ID document reference, payment methods (tokenised — no PAN/CVV),
//! and the user's notifications.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct UpdateMeReq { pub full_name: Option<String>, pub locale: Option<String> }

/// Right to rectification [M §10.1].
pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<UpdateMeReq>,
) -> AppResult<Json<Value>> {
    sqlx::query(
        "UPDATE user_account SET \
           full_name = COALESCE($2, full_name), \
           locale = COALESCE($3::locale_t, locale) \
         WHERE id = $1",
    )
    .bind(user.user_id).bind(req.full_name).bind(req.locale)
    .execute(&state.db).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Right to erasure [M §10.1]. Cascades remove the user's orders/tickets/etc.
pub async fn delete_me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    sqlx::query("DELETE FROM user_account WHERE id = $1").bind(user.user_id).execute(&state.db).await?;
    Ok(Json(json!({ "deleted": true })))
}

#[derive(Deserialize)]
pub struct IdDocReq { pub reference: String }

pub async fn set_id_document(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<IdDocReq>,
) -> AppResult<Json<Value>> {
    sqlx::query("UPDATE user_account SET id_document_ref = $2 WHERE id = $1")
        .bind(user.user_id).bind(&req.reference).execute(&state.db).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct PaymentMethodReq { pub scheme: String, pub gateway_token: String, pub last4: Option<String>, pub exp: Option<String> }

/// Store a tokenised payment method — only the gateway token + last4 [M §10.4].
pub async fn add_payment_method(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<PaymentMethodReq>,
) -> AppResult<Json<Value>> {
    if !["cib", "dahabia"].contains(&req.scheme.as_str()) {
        return Err(AppError::BadRequest("scheme must be cib or dahabia".into()));
    }
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO payment_method_token (user_id, scheme, gateway_token, last4, exp) \
         VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(user.user_id).bind(&req.scheme).bind(&req.gateway_token).bind(req.last4).bind(req.exp)
    .fetch_one(&state.db).await?;
    Ok(Json(json!({ "id": id })))
}

pub async fn list_payment_methods(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    let rows = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
        "SELECT id, scheme, last4 FROM payment_method_token WHERE user_id=$1 ORDER BY created_at DESC",
    ).bind(user.user_id).fetch_all(&state.db).await?;
    let items: Vec<Value> = rows.into_iter().map(|(id, scheme, last4)| json!({ "id": id, "scheme": scheme, "last4": last4 })).collect();
    Ok(Json(json!({ "methods": items })))
}

pub async fn notifications(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    let rows = sqlx::query_as::<_, (String, String, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT channel, subject, status, created_at FROM notification_log WHERE user_id=$1 ORDER BY created_at DESC LIMIT 50",
    ).bind(user.user_id).fetch_all(&state.db).await?;
    let items: Vec<Value> = rows.into_iter()
        .map(|(channel, subject, status, at)| json!({ "channel": channel, "subject": subject, "status": status, "at": at }))
        .collect();
    Ok(Json(json!({ "notifications": items })))
}
