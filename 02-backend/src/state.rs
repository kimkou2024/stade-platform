//! Shared application state passed to every handler.

use std::sync::Arc;

use crate::config::Config;
use crate::qr::QrSigner;
use crate::ratelimit::SharedRl;
use sqlx::postgres::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub cfg: Config,
    pub signer: Arc<QrSigner>,
    pub rl: SharedRl,
    pub redis: Option<redis::aio::ConnectionManager>,
}
