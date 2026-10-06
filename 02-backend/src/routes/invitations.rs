//! VIP invitations [M §8.4]: signed-QR invitations for tribunes A/B/C, with an
//! optional second-step check, plus a scan endpoint for the gate.

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::qr::{self, QrPayload};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateInvitationReq {
    pub tribune: String, // "A" | "B" | "C"
    pub holder_name: String,
    pub second_check_required: Option<bool>,
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
    Json(req): Json<CreateInvitationReq>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    if !["A", "B", "C"].contains(&req.tribune.as_str()) {
        return Err(AppError::BadRequest("tribune must be A, B or C".into()));
    }
    let id = Uuid::new_v4();
    let payload = QrPayload {
        v: 1, tid: id.to_string(), eid: event_id.to_string(), zid: format!("TRIBUNE-{}", req.tribune),
        typ: "vip_invitation".to_string(), iat: Utc::now().timestamp(), nonce: Uuid::new_v4().to_string(),
    };
    let token = state.signer.sign(&payload);
    let (payload_b64, sig_b64) = token.split_once('.').unwrap();
    sqlx::query(
        "INSERT INTO vip_invitation (id, event_id, tribune, holder_name, qr_payload, qr_signature, status, second_check_required) \
         VALUES ($1,$2,$3,$4,$5,$6,'valid',$7)",
    )
    .bind(id).bind(event_id).bind(&req.tribune).bind(&req.holder_name)
    .bind(payload_b64).bind(sig_b64).bind(req.second_check_required.unwrap_or(false))
    .execute(&state.db).await?;
    Ok(Json(json!({ "id": id, "tribune": req.tribune, "holder": req.holder_name, "qr": token })))
}

pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let rows = sqlx::query_as::<_, (Uuid, String, String, String, bool, String)>(
        "SELECT id, tribune, holder_name, status::text, second_check_required, \
                (qr_payload || '.' || qr_signature) AS qr \
         FROM vip_invitation WHERE event_id=$1 ORDER BY tribune, holder_name",
    )
    .bind(event_id).fetch_all(&state.db).await?;
    let items: Vec<Value> = rows.into_iter()
        .map(|(id, tribune, holder, status, second, qr)| json!({ "id": id, "tribune": tribune, "holder": holder, "status": status, "second_check_required": second, "qr": qr }))
        .collect();
    Ok(Json(json!({ "event_id": event_id, "invitations": items })))
}

#[derive(Deserialize)]
pub struct InvitationScanReq { pub token: String, pub gate_code: String, pub device_id: String }

/// Scan a VIP invitation [M §8.4]: verify signature, check state, mark used.
pub async fn scan(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<InvitationScanReq>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let payload = match qr::verify(&state.signer.public_key_b64(), &req.token) {
        Some(p) if p.typ == "vip_invitation" => p,
        _ => return Ok(Json(json!({ "result": "invalid" }))),
    };
    let id = Uuid::parse_str(&payload.tid).map_err(|_| AppError::BadRequest("bad id".into()))?;
    let mut tx = state.db.begin().await?;
    let row: Option<(String, String, bool)> = sqlx::query_as(
        "SELECT status::text, tribune, second_check_required FROM vip_invitation WHERE id=$1 FOR UPDATE",
    ).bind(id).fetch_optional(&mut *tx).await?;
    let result = match &row {
        None => "invalid",
        Some((status, _, _)) => match status.as_str() {
            "valid" => { sqlx::query("UPDATE vip_invitation SET status='used' WHERE id=$1").bind(id).execute(&mut *tx).await?; "valid" }
            "used" => "used",
            _ => "invalid",
        },
    };
    tx.commit().await?;
    let (tribune, second) = row.map(|(_, t, s)| (t, s)).unwrap_or_default();
    Ok(Json(json!({ "result": result, "tribune": tribune, "second_check_required": second, "gate": req.gate_code, "device": req.device_id })))
}
