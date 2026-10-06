//! Public catalog [M §7.1.1, §7.3]: events and live per-zone availability.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct EventRow {
    pub id: Uuid,
    pub title: String,
    pub kind: String,
    pub starts_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
}

pub async fn list_events(State(state): State<AppState>) -> AppResult<Json<Vec<EventRow>>> {
    let rows = sqlx::query_as::<_, EventRow>(
        "SELECT id, title, kind, starts_at, status::text AS status FROM event \
         WHERE status IN ('published','sales_open','sales_closed') ORDER BY starts_at",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

pub async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<EventRow>> {
    let row = sqlx::query_as::<_, EventRow>(
        "SELECT id, title, kind, starts_at, status::text AS status FROM event WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

/// Live availability per active zone: available = quota − issued − active holds.
pub async fn event_zones(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, rust_decimal::Decimal, i64, i64, i64)>(
        "SELECT z.id, z.code, z.name, ezc.price_dzd, ezc.quota::bigint, \
                COALESCE(t.issued,0) AS issued, COALESCE(h.held,0) AS held \
         FROM event_zone_config ezc \
         JOIN zone z ON z.id = ezc.zone_id \
         LEFT JOIN (SELECT zone_id, count(*) issued FROM ticket \
                    WHERE event_id = $1 AND status IN ('valid','used') GROUP BY zone_id) t \
                ON t.zone_id = ezc.zone_id \
         LEFT JOIN (SELECT zone_id, COALESCE(sum(qty),0) held FROM seat_lock \
                    WHERE event_id = $1 AND status = 'held' AND locked_until > now() \
                    GROUP BY zone_id) h ON h.zone_id = ezc.zone_id \
         WHERE ezc.event_id = $1 AND ezc.active = true \
         ORDER BY z.display_order",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let zones: Vec<Value> = rows
        .into_iter()
        .map(|(zid, code, name, price, quota, issued, held)| {
            let available = (quota - issued - held).max(0);
            json!({
                "zone_id": zid, "code": code, "name": name,
                "price_dzd": price, "quota": quota,
                "issued": issued, "held": held, "available": available
            })
        })
        .collect();
    Ok(Json(json!({ "event_id": id, "zones": zones })))
}
