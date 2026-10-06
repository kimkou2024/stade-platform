//! Purchase flow [M §7.1.4, §7.4, §7.1.6]: seat-lock → order → pay → issue
//! signed-QR tickets. Payment goes through the SATIM gateway (CIB/DAHABIA);
//! no PAN/CVV is stored [M §10.4]. In mock mode (no SATIM credentials) tickets
//! issue immediately; in SATIM mode the cardholder is redirected to the hosted
//! page and tickets issue on the confirmed return.

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::payments::{self, Mode};
use crate::qr::QrPayload;
use crate::state::AppState;

async fn available(state: &AppState, event_id: Uuid, zone_id: Uuid) -> AppResult<i64> {
    let row: Option<(i64, i64, i64)> = sqlx::query_as(
        "SELECT ezc.quota::bigint, \
                COALESCE((SELECT count(*) FROM ticket WHERE event_id=$1 AND zone_id=$2 \
                          AND status IN ('valid','used')),0), \
                COALESCE((SELECT sum(qty) FROM seat_lock WHERE event_id=$1 AND zone_id=$2 \
                          AND status='held' AND locked_until>now()),0) \
         FROM event_zone_config ezc WHERE ezc.event_id=$1 AND ezc.zone_id=$2 AND ezc.active",
    )
    .bind(event_id)
    .bind(zone_id)
    .fetch_optional(&state.db)
    .await?;
    let (quota, issued, held) = row.ok_or(AppError::BadRequest("zone not open for this event".into()))?;
    Ok(quota - issued - held)
}

#[derive(Deserialize)]
pub struct SeatLockReq { pub zone_id: Uuid, pub qty: i32 }

pub async fn seat_lock(
    State(state): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
    Json(req): Json<SeatLockReq>,
) -> AppResult<Json<Value>> {
    let max = state.cfg.max_tickets_per_order;
    if req.qty < 1 || req.qty > max {
        return Err(AppError::BadRequest(format!("qty must be 1..={max}")));
    }
    if available(&state, event_id, req.zone_id).await? < req.qty as i64 {
        return Err(AppError::BadRequest("not enough seats available".into()));
    }
    let until = Utc::now() + Duration::seconds(state.cfg.seat_lock_ttl_secs);
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO seat_lock (event_id, zone_id, user_id, qty, locked_until) \
         VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(event_id).bind(req.zone_id).bind(user.user_id).bind(req.qty).bind(until)
    .fetch_one(&state.db)
    .await?;
    // Mirror the hold into Redis with a TTL so it auto-expires there too [M §11.2].
    if let Some(mut cm) = state.redis.clone() {
        use redis::AsyncCommands;
        let _: Result<(), _> = cm
            .set_ex(format!("lock:{id}"), req.qty, state.cfg.seat_lock_ttl_secs.max(1) as u64)
            .await;
    }
    Ok(Json(json!({ "lock_id": id, "locked_until": until })))
}

#[derive(Deserialize)]
pub struct CreateOrderReq { pub lock_id: Uuid, pub holder_names: Vec<String> }

pub async fn create_order(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateOrderReq>,
) -> AppResult<Json<Value>> {
    let lock: Option<(Uuid, Uuid, i32, String)> = sqlx::query_as(
        "SELECT event_id, zone_id, qty, status::text FROM seat_lock \
         WHERE id=$1 AND user_id=$2 AND locked_until>now()",
    )
    .bind(req.lock_id).bind(user.user_id).fetch_optional(&state.db).await?;
    let (event_id, zone_id, qty, status) = lock.ok_or(AppError::BadRequest("lock expired or not found".into()))?;
    if status != "held" { return Err(AppError::BadRequest("lock already used".into())); }
    if req.holder_names.len() as i32 != qty { return Err(AppError::BadRequest("holder_names count must equal locked qty".into())); }

    // Anti-fraud: too many orders from one account in a short window [M §10.5].
    let recent: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM \"order\" WHERE user_id=$1 AND created_at > now() - interval '1 hour'",
    ).bind(user.user_id).fetch_one(&state.db).await?;
    if recent >= 20 {
        sqlx::query("INSERT INTO fraud_event (type, user_id, detail) VALUES ('multi_purchase',$1,$2)")
            .bind(user.user_id).bind(json!({ "orders_last_hour": recent })).execute(&state.db).await?;
        if recent >= 50 {
            sqlx::query("UPDATE user_account SET status='blacklisted' WHERE id=$1").bind(user.user_id).execute(&state.db).await?;
            return Err(AppError::BadRequest("account temporarily blocked for review".into()));
        }
    }
    let price: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT price_dzd FROM event_zone_config WHERE event_id=$1 AND zone_id=$2",
    ).bind(event_id).bind(zone_id).fetch_one(&state.db).await?;
    let total = price * rust_decimal::Decimal::from(qty);
    let order_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO \"order\" (user_id, event_id, total_dzd, status) VALUES ($1,$2,$3,'pending') RETURNING id",
    ).bind(user.user_id).bind(event_id).bind(total).fetch_one(&state.db).await?;
    Ok(Json(json!({ "order_id": order_id, "event_id": event_id, "zone_id": zone_id, "qty": qty, "total_dzd": total, "status": "pending" })))
}

#[derive(Deserialize)]
pub struct PayReq { pub lock_id: Uuid, pub holder_names: Vec<String>, pub gateway: Option<String> }

/// Issue one signed-QR ticket per holder within a transaction [M §7.1.6].
async fn issue_tickets(
    tx: &mut Transaction<'_, Postgres>,
    signer: &crate::qr::QrSigner,
    order_id: Uuid,
    event_id: Uuid,
    zone_id: Uuid,
    holder_names: &[String],
) -> AppResult<Vec<Value>> {
    let mut tickets = Vec::new();
    for name in holder_names {
        let tid = Uuid::new_v4();
        let payload = QrPayload {
            v: 1, tid: tid.to_string(), eid: event_id.to_string(), zid: zone_id.to_string(),
            typ: "local".to_string(), iat: Utc::now().timestamp(), nonce: Uuid::new_v4().to_string(),
        };
        let token = signer.sign(&payload);
        let (payload_b64, sig_b64) = token.split_once('.').unwrap();
        sqlx::query(
            "INSERT INTO ticket (id, order_id, event_id, zone_id, holder_name, type, qr_payload, qr_signature, status) \
             VALUES ($1,$2,$3,$4,$5,'local',$6,$7,'valid')",
        )
        .bind(tid).bind(order_id).bind(event_id).bind(zone_id).bind(name).bind(payload_b64).bind(sig_b64)
        .execute(&mut **tx).await?;
        tickets.push(json!({ "ticket_id": tid, "holder": name, "qr": token }));
    }
    Ok(tickets)
}

pub async fn pay_order(
    State(state): State<AppState>,
    user: AuthUser,
    Path(order_id): Path<Uuid>,
    Json(req): Json<PayReq>,
) -> AppResult<Json<Value>> {
    let mut tx = state.db.begin().await?;
    let order: Option<(Uuid, rust_decimal::Decimal, String)> = sqlx::query_as(
        "SELECT event_id, total_dzd, status::text FROM \"order\" WHERE id=$1 AND user_id=$2 FOR UPDATE",
    ).bind(order_id).bind(user.user_id).fetch_optional(&mut *tx).await?;
    let (event_id, total, status) = order.ok_or(AppError::NotFound)?;
    if status != "pending" { return Err(AppError::BadRequest("order not payable".into())); }

    let lock: Option<(Uuid, i32)> = sqlx::query_as(
        "SELECT zone_id, qty FROM seat_lock WHERE id=$1 AND user_id=$2 AND status='held' FOR UPDATE",
    ).bind(req.lock_id).bind(user.user_id).fetch_optional(&mut *tx).await?;
    let (zone_id, qty) = lock.ok_or(AppError::BadRequest("lock expired".into()))?;
    if req.holder_names.len() as i32 != qty { return Err(AppError::BadRequest("holder_names count must equal qty".into())); }
    let gateway = req.gateway.clone().unwrap_or_else(|| "cib".to_string());

    match payments::mode(&state.cfg) {
        Mode::Satim => {
            // Register with SATIM and hand back the hosted-page URL. Tickets are
            // issued later by satim_return once payment is confirmed.
            let (satim_order_id, form_url) = payments::register(&state.cfg, &order_id.to_string(), total).await?;
            sqlx::query(
                "INSERT INTO payment_transaction (order_id, gateway, amount_dzd, status, gateway_txn_ref, raw_callback) \
                 VALUES ($1,$2,$3,'pending',$4,$5)",
            )
            .bind(order_id).bind(&gateway).bind(total).bind(&satim_order_id)
            .bind(json!({ "lock_id": req.lock_id, "zone_id": zone_id, "holder_names": req.holder_names }))
            .execute(&mut *tx).await?;
            tx.commit().await?;
            Ok(Json(json!({ "status": "redirect", "form_url": form_url, "satim_order_id": satim_order_id })))
        }
        Mode::Mock => {
            sqlx::query(
                "INSERT INTO payment_transaction (order_id, gateway, amount_dzd, status, gateway_txn_ref) \
                 VALUES ($1,$2,$3,'accepted',$4)",
            )
            .bind(order_id).bind(&gateway).bind(total).bind(format!("MOCK-{}", Uuid::new_v4()))
            .execute(&mut *tx).await?;
            let tickets = issue_tickets(&mut tx, &state.signer, order_id, event_id, zone_id, &req.holder_names).await?;
            sqlx::query("UPDATE \"order\" SET status='paid' WHERE id=$1").bind(order_id).execute(&mut *tx).await?;
            sqlx::query("UPDATE seat_lock SET status='converted' WHERE id=$1").bind(req.lock_id).execute(&mut *tx).await?;
            tx.commit().await?;
            crate::notify::send(&state, Some(user.user_id), "email", "Confirmation d'achat",
                &format!("Votre paiement est accepté. {} billet(s) émis.", tickets.len())).await;
            Ok(Json(json!({ "order_id": order_id, "status": "paid", "tickets": tickets })))
        }
    }
}

#[derive(Deserialize)]
pub struct SatimReturn { #[serde(rename = "orderId")] pub order_id: String }

/// SATIM return/callback [M §7.4]: confirm the order status, and on success
/// issue the signed-QR tickets. Called by the browser return URL after payment.
pub async fn satim_return(
    State(state): State<AppState>,
    Query(q): Query<SatimReturn>,
) -> AppResult<Json<Value>> {
    // Locate the pending transaction by SATIM order id.
    let row: Option<(Uuid, Value)> = sqlx::query_as(
        "SELECT order_id, raw_callback FROM payment_transaction \
         WHERE gateway_txn_ref=$1 AND status='pending' ORDER BY created_at DESC LIMIT 1",
    ).bind(&q.order_id).fetch_optional(&state.db).await?;
    let (order_id, meta) = row.ok_or(AppError::NotFound)?;

    let paid = payments::is_paid(&state.cfg, &q.order_id).await?;
    if !paid {
        sqlx::query("UPDATE payment_transaction SET status='failed' WHERE gateway_txn_ref=$1").bind(&q.order_id).execute(&state.db).await?;
        return Ok(Json(json!({ "status": "failed" })));
    }

    let lock_id: Uuid = serde_json::from_value(meta["lock_id"].clone()).map_err(|_| AppError::Internal)?;
    let zone_id: Uuid = serde_json::from_value(meta["zone_id"].clone()).map_err(|_| AppError::Internal)?;
    let holder_names: Vec<String> = serde_json::from_value(meta["holder_names"].clone()).map_err(|_| AppError::Internal)?;
    let event_id: Uuid = sqlx::query_scalar("SELECT event_id FROM \"order\" WHERE id=$1").bind(order_id).fetch_one(&state.db).await?;

    let mut tx = state.db.begin().await?;
    let tickets = issue_tickets(&mut tx, &state.signer, order_id, event_id, zone_id, &holder_names).await?;
    sqlx::query("UPDATE payment_transaction SET status='accepted' WHERE gateway_txn_ref=$1").bind(&q.order_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE \"order\" SET status='paid' WHERE id=$1").bind(order_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE seat_lock SET status='converted' WHERE id=$1").bind(lock_id).execute(&mut *tx).await?;
    tx.commit().await?;
    let uid: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM \"order\" WHERE id=$1").bind(order_id).fetch_optional(&state.db).await?;
    crate::notify::send(&state, uid, "email", "Confirmation d'achat",
        &format!("Votre paiement est accepté. {} billet(s) émis.", tickets.len())).await;
    Ok(Json(json!({ "status": "paid", "order_id": order_id, "tickets": tickets })))
}

/// Cashier/e-ticket PDF with a scannable QR [M §7.1.5]. Owner or staff.
pub async fn ticket_pdf(
    State(state): State<AppState>,
    user: AuthUser,
    Path(ticket_id): Path<Uuid>,
) -> AppResult<axum::response::Response> {
    use axum::http::header;
    use axum::response::IntoResponse;
    let row: Option<(Uuid, String, String, String, String, String)> = sqlx::query_as(
        "SELECT o.user_id, e.title, t.holder_name, z.name, \
                (t.qr_payload || '.' || t.qr_signature), t.id::text \
         FROM ticket t JOIN \"order\" o ON o.id=t.order_id \
         JOIN event e ON e.id=t.event_id JOIN zone z ON z.id=t.zone_id \
         WHERE t.id=$1",
    ).bind(ticket_id).fetch_optional(&state.db).await?;
    let (owner, event, holder, zone, qr, tid) = row.ok_or(AppError::NotFound)?;
    let is_staff = matches!(user.role.as_str(), "admin" | "operator" | "gate_agent");
    if owner != user.user_id && !is_staff { return Err(AppError::Unauthorized); }
    let bytes = crate::pdf::cashier_ticket(&event, &holder, &zone, &tid, &qr);
    Ok((
        [(header::CONTENT_TYPE, "application/pdf"),
         (header::CONTENT_DISPOSITION, "attachment; filename=\"billet.pdf\"")],
        bytes,
    ).into_response())
}

pub async fn my_tickets(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String)>(
        "SELECT t.id, t.event_id, t.holder_name, t.status::text, (t.qr_payload || '.' || t.qr_signature) AS qr \
         FROM ticket t JOIN \"order\" o ON o.id = t.order_id \
         WHERE o.user_id = $1 ORDER BY t.issued_at DESC",
    ).bind(user.user_id).fetch_all(&state.db).await?;
    let tickets: Vec<Value> = rows.into_iter()
        .map(|(id, eid, holder, status, qr)| json!({ "ticket_id": id, "event_id": eid, "holder": holder, "status": status, "qr": qr }))
        .collect();
    Ok(Json(json!({ "tickets": tickets })))
}
