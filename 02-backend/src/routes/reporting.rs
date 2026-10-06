//! Reporting & supervision [M §8.5, §9.3].

use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ExportQuery { pub format: Option<String> }

/// Export the spectator/ticket list as CSV or PDF [M §9.2].
pub async fn export_spectators(
    State(state): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
    Query(q): Query<ExportQuery>,
) -> AppResult<Response> {
    user.require_staff()?;
    let rows = sqlx::query_as::<_, (Uuid, String, String, String)>(
        "SELECT t.id, t.holder_name, z.name, t.status::text FROM ticket t \
         JOIN zone z ON z.id = t.zone_id WHERE t.event_id=$1 ORDER BY z.name, t.holder_name",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;

    if q.format.as_deref() == Some("pdf") {
        let title: String = sqlx::query_scalar("SELECT title FROM event WHERE id=$1")
            .bind(event_id).fetch_optional(&state.db).await?.unwrap_or_else(|| "Spectateurs".into());
        let list: Vec<(String, String, String)> = rows.into_iter().map(|(_, h, z, s)| (h, z, s)).collect();
        let bytes = crate::pdf::spectator_list(&format!("Spectateurs — {title}"), &list);
        return Ok((
            [(header::CONTENT_TYPE, "application/pdf"),
             (header::CONTENT_DISPOSITION, "attachment; filename=\"spectators.pdf\"")],
            bytes,
        ).into_response());
    }

    let mut csv = String::from("ticket_id,holder_name,zone,status\n");
    for (id, holder, zone, status) in rows {
        let esc = |s: String| format!("\"{}\"", s.replace('"', "\"\""));
        csv.push_str(&format!("{},{},{},{}\n", id, esc(holder), esc(zone), status));
    }
    Ok((
        [(header::CONTENT_TYPE, "text/csv; charset=utf-8"),
         (header::CONTENT_DISPOSITION, "attachment; filename=\"spectators.csv\"")],
        csv,
    ).into_response())
}

/// Sales summary for an event: tickets sold and revenue per zone [M §9.3].
pub async fn sales_report(
    State(state): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let rows = sqlx::query_as::<_, (String, i64, rust_decimal::Decimal)>(
        "SELECT z.name, count(t.id), COALESCE(sum(ezc.price_dzd),0) \
         FROM ticket t \
         JOIN zone z ON z.id = t.zone_id \
         JOIN event_zone_config ezc ON ezc.event_id = t.event_id AND ezc.zone_id = t.zone_id \
         WHERE t.event_id = $1 AND t.status IN ('valid','used') \
         GROUP BY z.name ORDER BY z.name",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;
    let by_zone: Vec<Value> = rows
        .iter()
        .map(|(name, cnt, rev)| json!({ "zone": name, "sold": cnt, "revenue_dzd": rev }))
        .collect();
    let total_sold: i64 = rows.iter().map(|(_, c, _)| c).sum();
    Ok(Json(json!({ "event_id": event_id, "total_sold": total_sold, "by_zone": by_zone })))
}

/// Live attendance dashboard: entries per zone and invalid-scan rate [M §8.5].
pub async fn dashboard(
    State(state): State<AppState>,
    user: AuthUser,
    Path(event_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    user.require_staff()?;
    let entries: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM access_log WHERE event_id=$1 AND result='valid'",
    )
    .bind(event_id)
    .fetch_one(&state.db)
    .await?;
    let invalid: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM access_log WHERE event_id=$1 AND result<>'valid'",
    )
    .bind(event_id)
    .fetch_one(&state.db)
    .await?;
    let per_gate = sqlx::query_as::<_, (String, i64)>(
        "SELECT gate_code, count(*) FROM access_log \
         WHERE event_id=$1 AND result='valid' GROUP BY gate_code ORDER BY gate_code",
    )
    .bind(event_id)
    .fetch_all(&state.db)
    .await?;
    let gates: Vec<Value> = per_gate.into_iter().map(|(g, c)| json!({ "gate": g, "entries": c })).collect();

    // Automatic alerts [M §8.5].
    let mut alerts: Vec<Value> = Vec::new();
    // Per-zone capacity overflow: valid entries vs configured quota.
    let over = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT z.name, COALESCE(ezc.quota,0)::bigint, count(al.id) \
         FROM event_zone_config ezc JOIN zone z ON z.id=ezc.zone_id \
         LEFT JOIN ticket t ON t.event_id=ezc.event_id AND t.zone_id=ezc.zone_id \
         LEFT JOIN access_log al ON al.ticket_id=t.id AND al.result='valid' \
         WHERE ezc.event_id=$1 GROUP BY z.name, ezc.quota HAVING count(al.id) > COALESCE(ezc.quota,0)",
    ).bind(event_id).fetch_all(&state.db).await.unwrap_or_default();
    for (zone, quota, cnt) in over {
        alerts.push(json!({ "type": "capacity_exceeded", "zone": zone, "quota": quota, "entries": cnt }));
    }
    // Abnormally high invalid-scan rate.
    let total = entries + invalid;
    if total >= 20 && (invalid as f64) / (total as f64) > 0.2 {
        alerts.push(json!({ "type": "high_invalid_rate", "invalid": invalid, "total": total }));
    }
    // Persist any alerts (best-effort) so they are auditable.
    for a in &alerts {
        let _ = sqlx::query("INSERT INTO access_alert (event_id, type, detail) VALUES ($1,$2,$3)")
            .bind(event_id).bind(a["type"].as_str().unwrap_or("alert")).bind(a).execute(&state.db).await;
    }

    Ok(Json(json!({
        "event_id": event_id, "entries": entries, "invalid_scans": invalid, "per_gate": gates, "alerts": alerts
    })))
}
