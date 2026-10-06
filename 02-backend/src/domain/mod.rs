//! Domain types mapped from the database via sqlx `FromRow`.

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Zone {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    #[sqlx(rename = "kind")]
    pub kind: String,
    pub sellable: bool,
    pub display_order: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TribuneSection {
    pub id: Uuid,
    pub zone_id: Uuid,
    pub label: String,
    pub level: Option<String>,
    pub gate_range: Option<String>,
    pub capacity: i32,
    pub sellable: bool,
}
