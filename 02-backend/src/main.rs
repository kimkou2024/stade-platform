//! Stade Ali Ammar — e-ticketing & access-control backend.
//! Imposed stack [M §11.1]: Rust / Axum / Tokio + PostgreSQL + Redis.

mod auth;
mod config;
mod domain;
mod error;
mod notify;
mod payments;
mod pdf;
mod qr;
mod ratelimit;
mod routes;
// (routes submodules are declared in routes/mod.rs)
mod state;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::middleware::{self, Next};
use axum::response::Response;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::error::AppError;
use crate::qr::QrSigner;
use crate::ratelimit::{Limits, RlState};
use crate::state::AppState;

fn limits(cfg: &Config) -> Limits {
    Limits {
        window: Duration::from_secs(cfg.rate_window_secs.max(1) as u64),
        max: cfg.rate_max.max(1) as usize,
        fail_window: Duration::from_secs(cfg.login_fail_window_secs.max(1) as u64),
        fail_max: cfg.login_fail_max.max(1) as u32,
    }
}

/// Per-key sliding-window rate limit [M §10.3]. Key = client IP (X-Forwarded-For)
/// or a shared bucket when unknown.
async fn rate_limit_mw(State(state): State<AppState>, req: Request, next: Next) -> Result<Response, AppError> {
    let key = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "global".to_string());
    let lim = limits(&state.cfg);
    let ok = state.rl.lock().map(|mut g| g.allow(&key, &lim, Instant::now())).unwrap_or(true);
    if !ok { return Err(AppError::TooManyRequests); }
    Ok(next.run(req).await)
}

#[tokio::main]
async fn main() -> anyhow_lite::Result {
    // Logging [C2 §4.4]
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("stade_backend=info,tower_http=info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let _ = dotenvy::dotenv();
    let cfg = Config::from_env();

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await
        .map_err(|e| format!("db connect failed: {e}"))?;

    // Apply migrations on startup (dev convenience).
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .map_err(|e| format!("migration failed: {e}"))?;

    // QR signing key [M §7.1.6]. Set QR_SIGNING_SEED (base64, 32 bytes) in prod.
    let signer = Arc::new(QrSigner::from_seed_or_random(
        std::env::var("QR_SIGNING_SEED").ok().as_deref(),
    ));

    // Optional Redis for real-time counters / seat-lock TTL [M §11.2].
    let redis = match redis::Client::open(cfg.redis_url.clone()) {
        Ok(client) => match redis::aio::ConnectionManager::new(client).await {
            Ok(cm) => { tracing::info!("redis connected"); Some(cm) }
            Err(e) => { tracing::warn!("redis unavailable ({e}); using PostgreSQL only"); None }
        },
        Err(e) => { tracing::warn!("redis url invalid ({e})"); None }
    };

    let bind = cfg.bind_addr.clone();
    let state = AppState {
        db, cfg, signer,
        rl: Arc::new(Mutex::new(RlState::default())),
        redis,
    };
    let app = routes::router(state.clone())
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_mw))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .map_err(|e| format!("bind {bind} failed: {e}"))?;
    tracing::info!("stade-backend listening on {bind}");
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("server error: {e}"))?;
    Ok(())
}

/// Tiny local error alias so `main` can use `?` on `String` errors without
/// pulling in the `anyhow` crate.
mod anyhow_lite {
    pub type Result = std::result::Result<(), Box<dyn std::error::Error>>;
}
