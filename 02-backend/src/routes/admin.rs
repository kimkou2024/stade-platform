//! Back-office event administration [M §7.2, §9.2]. Staff-only (RBAC coarse
//! gate for now; fine-grained permissions layered later).

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct BootstrapReq { pub email: String, pub role: Option<String> }

/// Promote a user to a staff role. Guarded by the X-Bootstrap-Token header
/// matching BOOTSTRAP_TOKEN — used once to create the first admin, then the
/// admin manages everyone else [M §9.1]. Disabled when the token is unset.
pub async fn bootstrap(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<BootstrapReq>,
) -> AppResult<Json<Value>> {
    let token = state.cfg.bootstrap_token.clone();
    if token.is_empty() { return Err(AppError::Unauthorized); }
    let provided = headers.get("x-bootstrap-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    if provided != token { return Err(AppError::Unauthorized); }

    let role = req.role.unwrap_or_else(|| "admin".to_string());
    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM user_account WHERE email=$1")
        .bind(&req.email)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::BadRequest("no user with that email".into()))?;

    let mut tx = state.db.begin().await?;
    let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM agent WHERE user_id=$1")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;
    let agent_id: Uuid = match existing {
        Some(id) => id,
        None => sqlx::query_scalar("INSERT INTO agent (user_id) VALUES ($1) RETURNING id")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?,
    };
    let role_id: Uuid = sqlx::query_scalar("SELECT id FROM role WHERE code=$1")
        .bind(&role)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::BadRequest("unknown role".into()))?;
    sqlx::query("INSERT INTO agent_role (agent_id, role_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
        .bind(agent_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "email": req.email, "role": role, "ok": true })))
}

/// List every event (all statuses) for the back-office [M §9.2]. The public
/// `/api/events` only returns published ones, so staff get this to see drafts.
pub async fn list_events(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let rows = sqlx::query_as::<_, (Uuid, String, String, chrono::DateTime<chrono::Utc>, String)>(
        "SELECT id, title, kind, starts_at, status::text FROM event ORDER BY starts_at DESC",
    )
    .fetch_all(&state.db)
    .await?;
    let events: Vec<Value> = rows.into_iter()
        .map(|(id, title, kind, starts_at, status)| json!({ "id": id, "title": title, "kind": kind, "starts_at": starts_at, "status": status }))
        .collect();
    Ok(Json(json!({ "events": events })))
}

#[derive(Deserialize)]
pub struct CreateEventReq {
    pub title: String,
    pub starts_at: chrono::DateTime<chrono::Utc>,
    pub kind: Option<String>,
    pub capacity: Option<i32>,
    pub entry_conditions: Option<String>,
}

pub async fn create_event(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateEventReq>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO event (title, starts_at, kind, capacity, entry_conditions, created_by) \
         VALUES ($1,$2,COALESCE($3,'match'),$4,$5,$6) RETURNING id",
    )
    .bind(&req.title)
    .bind(req.starts_at)
    .bind(req.kind)
    .bind(req.capacity)
    .bind(req.entry_conditions)
    .bind(user.user_id)
    .fetch_one(&state.db)
    .await?;
    audit(&state, user.user_id, "event.create", "event", Some(id), json!({ "title": req.title })).await;
    Ok(Json(json!({ "id": id, "status": "draft" })))
}

#[derive(Deserialize)]
pub struct ZoneConfigReq {
    pub zone_id: Uuid,
    pub quota: i32,
    pub price_dzd: rust_decimal::Decimal,
    pub ticket_category: String, // local|visitor|vip|vvip|official
    pub active: Option<bool>,
}

pub async fn configure_zone(
    State(state): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
    Json(req): Json<ZoneConfigReq>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    sqlx::query(
        "INSERT INTO event_zone_config (event_id, zone_id, quota, price_dzd, ticket_category, active) \
         VALUES ($1,$2,$3,$4,$5::ticket_category,COALESCE($6,true)) \
         ON CONFLICT (event_id, zone_id) DO UPDATE \
           SET quota = EXCLUDED.quota, price_dzd = EXCLUDED.price_dzd, \
               ticket_category = EXCLUDED.ticket_category, active = EXCLUDED.active",
    )
    .bind(event_id)
    .bind(req.zone_id)
    .bind(req.quota)
    .bind(req.price_dzd)
    .bind(&req.ticket_category)
    .bind(req.active)
    .execute(&state.db)
    .await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct StatusReq {
    pub status: String, // draft|published|sales_open|sales_closed|closed|cancelled|postponed
}

pub async fn set_event_status(
    State(state): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
    Json(req): Json<StatusReq>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let n = sqlx::query("UPDATE event SET status = $1::event_status WHERE id = $2")
        .bind(&req.status)
        .bind(event_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(AppError::NotFound);
    }
    audit(&state, user.user_id, "event.status", "event", Some(event_id), json!({ "status": req.status })).await;
    Ok(Json(json!({ "id": event_id, "status": req.status })))
}

/// Write an audit entry [M §9.1]. Best-effort: never fails the request.
async fn audit(state: &AppState, user_id: Uuid, action: &str, entity: &str, entity_id: Option<Uuid>, after: Value) {
    let agent_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM agent WHERE user_id=$1")
        .bind(user_id).fetch_optional(&state.db).await.ok().flatten();
    let _ = sqlx::query(
        "INSERT INTO audit_log (agent_id, action, entity, entity_id, after) VALUES ($1,$2,$3,$4,$5)",
    ).bind(agent_id).bind(action).bind(entity).bind(entity_id).bind(after).execute(&state.db).await;
}

pub async fn audit_log(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let rows = sqlx::query_as::<_, (String, String, Option<Uuid>, chrono::DateTime<chrono::Utc>)>(
        "SELECT action, entity, entity_id, occurred_at FROM audit_log ORDER BY occurred_at DESC LIMIT 100",
    ).fetch_all(&state.db).await?;
    let items: Vec<Value> = rows.into_iter()
        .map(|(action, entity, entity_id, at)| json!({ "action": action, "entity": entity, "entity_id": entity_id, "at": at }))
        .collect();
    Ok(Json(json!({ "entries": items })))
}

// ── Data retention purge [M §10.1] ────────────────────────────────────────
#[derive(Deserialize)]
pub struct PurgeReq { pub days: Option<i64> }

/// Purge expired/stale personal and transient data after the retention term
/// [M §10.1]. Run periodically (e.g. a daily scheduled job).
pub async fn retention_purge(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<PurgeReq>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let days = req.days.unwrap_or(365).max(1);
    let interval = format!("{days} days");
    let resets = sqlx::query("DELETE FROM password_reset WHERE expires_at < now()")
        .execute(&state.db).await?.rows_affected();
    let notes = sqlx::query("DELETE FROM notification_log WHERE created_at < now() - $1::interval")
        .bind(&interval).execute(&state.db).await?.rows_affected();
    let locks = sqlx::query("DELETE FROM seat_lock WHERE status <> 'converted' AND locked_until < now()")
        .execute(&state.db).await?.rows_affected();
    audit(&state, user.user_id, "retention.purge", "system", None, json!({ "days": days })).await;
    Ok(Json(json!({ "purged": { "password_resets": resets, "notifications": notes, "expired_locks": locks } })))
}

// ── Subscriptions [M §7.5] ────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct CreateSubscriptionReq { pub email: String, pub season: String, pub zone_code: Option<String> }

pub async fn create_subscription(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateSubscriptionReq>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM user_account WHERE email=$1")
        .bind(&req.email).fetch_optional(&state.db).await?
        .ok_or_else(|| AppError::BadRequest("no user with that email".into()))?;
    let zone_id: Option<Uuid> = match &req.zone_code {
        Some(code) => sqlx::query_scalar("SELECT id FROM zone WHERE code=$1").bind(code).fetch_optional(&state.db).await?,
        None => None,
    };
    let id = Uuid::new_v4();
    let card_ref = format!("SUB-{}", id.simple());
    sqlx::query(
        "INSERT INTO subscription (id, user_id, season, card_ref, zone_id, status) VALUES ($1,$2,$3,$4,$5,'active')",
    ).bind(id).bind(user_id).bind(&req.season).bind(&card_ref).bind(zone_id).execute(&state.db).await?;
    // Signed QR derived from the subscription id (not persisted; verifiable offline).
    let token = state.signer.sign(&crate::qr::QrPayload {
        v: 1, tid: id.to_string(), eid: String::new(), zid: req.zone_code.unwrap_or_default(),
        typ: "subscription".to_string(), iat: chrono::Utc::now().timestamp(), nonce: Uuid::new_v4().to_string(),
    });
    Ok(Json(json!({ "id": id, "card_ref": card_ref, "season": req.season, "qr": token })))
}

pub async fn list_subscriptions(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let rows = sqlx::query_as::<_, (Uuid, String, String, String)>(
        "SELECT s.id, s.season, s.card_ref, u.email FROM subscription s \
         JOIN user_account u ON u.id = s.user_id ORDER BY s.issued_at DESC",
    ).fetch_all(&state.db).await?;
    let items: Vec<Value> = rows.into_iter()
        .map(|(id, season, card, email)| json!({ "id": id, "season": season, "card_ref": card, "email": email }))
        .collect();
    Ok(Json(json!({ "subscriptions": items })))
}
