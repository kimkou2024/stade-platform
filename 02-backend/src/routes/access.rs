//! Access control [M §8.1, §8.2, §8.3]: offline manifest, online scan, and
//! batch sync with double-scan conflict resolution.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::qr;
use crate::state::AppState;

/// Public key gates use to verify QR offline [M §8.2].
pub async fn public_key(State(state): State<AppState>) -> Json<Value> {
    Json(json!({ "alg": "ed25519", "public_key": state.signer.public_key_b64() }))
}

/// Pre-sync snapshot of valid tickets for an event [M §8.2].
pub async fn manifest(
    State(state): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String)>(
        "SELECT id, zone_id, status::text, (qr_payload || '.' || qr_signature) \
         FROM ticket WHERE event_id = $1",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;
    let tickets: Vec<Value> = rows
        .into_iter()
        .map(|(id, zid, status, qr)| json!({ "ticket_id": id, "zone_id": zid, "status": status, "qr": qr }))
        .collect();
    Ok(Json(json!({
        "event_id": event_id,
        "public_key": state.signer.public_key_b64(),
        "count": tickets.len(),
        "tickets": tickets
    })))
}

#[derive(Deserialize)]
pub struct ScanReq {
    pub token: String, // the scanned QR string
    pub gate_code: String,
    pub device_id: String,
}

/// Online scan: verify signature, check state, mark used, log [M §8.1, §8.3].
pub async fn scan(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<ScanReq>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let pk = state.signer.public_key_b64();
    let payload = match qr::verify(&pk, &req.token) {
        Some(p) => p,
        None => return Ok(Json(json!({ "result": "invalid", "reason": "bad signature" }))),
    };
    let ticket_id = Uuid::parse_str(&payload.tid).map_err(|_| AppError::BadRequest("bad ticket id".into()))?;

    let mut tx = state.db.begin().await?;
    let row: Option<(String,)> =
        sqlx::query_as("SELECT status::text FROM ticket WHERE id=$1 FOR UPDATE")
            .bind(ticket_id)
            .fetch_optional(&mut *tx)
            .await?;
    let result = match row {
        None => "invalid",
        Some((status,)) => match status.as_str() {
            "valid" => {
                sqlx::query("UPDATE ticket SET status='used', used_at=now(), used_gate=$2 WHERE id=$1")
                    .bind(ticket_id)
                    .bind(&req.gate_code)
                    .execute(&mut *tx)
                    .await?;
                "valid"
            }
            "used" => "used",
            "cancelled" => "cancelled",
            "blocked" => "blocked",
            _ => "invalid",
        },
    };
    sqlx::query(
        "INSERT INTO access_log (ticket_id, event_id, gate_code, device_id, result, scanned_at, synced, source) \
         VALUES ($1,$2,$3,$4,$5::ticket_status,now(),true,'online')",
    )
    .bind(if result == "invalid" { None } else { Some(ticket_id) })
    .bind(Uuid::parse_str(&payload.eid).ok())
    .bind(&req.gate_code)
    .bind(&req.device_id)
    .bind(result)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "result": result, "ticket_id": ticket_id })))
}

#[derive(Deserialize)]
pub struct OfflineEntry {
    pub ticket_id: Uuid,
    pub event_id: Uuid,
    pub gate_code: String,
    pub device_id: String,
    pub scanned_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct SyncReq {
    pub entries: Vec<OfflineEntry>,
}

/// Batch resync of offline entries [M §8.2]. The partial unique index
/// `one_valid_entry_per_ticket` enforces the double-scan rule: the first
/// accepted entry wins; a later one at another gate is logged as fraud.
pub async fn sync(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<SyncReq>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let mut accepted = 0u32;
    let mut conflicts = 0u32;
    for e in &req.entries {
        let mut tx = state.db.begin().await?;
        let ins = sqlx::query(
            "INSERT INTO access_log (ticket_id, event_id, gate_code, device_id, result, scanned_at, synced, source) \
             VALUES ($1,$2,$3,$4,'valid',$5,true,'offline')",
        )
        .bind(e.ticket_id)
        .bind(e.event_id)
        .bind(&e.gate_code)
        .bind(&e.device_id)
        .bind(e.scanned_at)
        .execute(&mut *tx)
        .await;
        match ins {
            Ok(_) => {
                sqlx::query(
                    "UPDATE ticket SET status='used', used_at=$2, used_gate=$3 \
                     WHERE id=$1 AND status='valid'",
                )
                .bind(e.ticket_id)
                .bind(e.scanned_at)
                .bind(&e.gate_code)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                accepted += 1;
            }
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
                // Duplicate entry for an already-used ticket → fraud event.
                tx.rollback().await?;
                sqlx::query(
                    "INSERT INTO fraud_event (type, detail) VALUES ('duplicate', $1)",
                )
                .bind(json!({
                    "ticket_id": e.ticket_id, "gate": e.gate_code,
                    "device": e.device_id, "scanned_at": e.scanned_at
                }))
                .execute(&state.db)
                .await?;
                conflicts += 1;
            }
            Err(other) => {
                tx.rollback().await?;
                return Err(AppError::from(other));
            }
        }
    }
    Ok(Json(json!({ "accepted": accepted, "conflicts": conflicts })))
}
