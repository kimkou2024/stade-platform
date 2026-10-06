//! Reference seating endpoints [M §7.3][X]. Per-event availability
//! (`GET /events/{id}/zones`) is added with the catalog module.

use axum::extract::State;
use axum::Json;

use crate::domain::{TribuneSection, Zone};
use crate::error::AppResult;
use crate::state::AppState;

pub async fn list_zones(State(state): State<AppState>) -> AppResult<Json<Vec<Zone>>> {
    // kind::text so the Postgres enum decodes into String.
    let zones = sqlx::query_as::<_, Zone>(
        "SELECT id, code, name, kind::text AS kind, sellable, display_order \
         FROM zone ORDER BY display_order",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(zones))
}

pub async fn list_sections(State(state): State<AppState>) -> AppResult<Json<Vec<TribuneSection>>> {
    let sections = sqlx::query_as::<_, TribuneSection>(
        "SELECT id, zone_id, label, level, gate_range, capacity, sellable \
         FROM tribune_section ORDER BY label",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(sections))
}
