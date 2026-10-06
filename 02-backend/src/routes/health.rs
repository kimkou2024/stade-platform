//! Liveness/readiness. Readiness pings the DB — useful for the C2 monitoring
//! requirement [C2 §4.4].

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::error::AppResult;
use crate::state::AppState;

pub async fn health(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();
    // Redis is optional; report whether a connection is configured and live.
    let redis_ok = if let Some(mut cm) = state.redis.clone() {
        let r: Result<String, _> = redis::cmd("PING").query_async(&mut cm).await;
        r.is_ok()
    } else {
        false
    };
    Ok(Json(json!({
        "status": "ok",
        "db": db_ok,
        "redis": redis_ok,
    })))
}
