//! Environment configuration. See `.env.example`.

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub bind_addr: String,
    pub jwt_secret: String,
    pub jwt_access_ttl_secs: i64,
    pub jwt_refresh_ttl_secs: i64,
    pub qr_scheme: String,
    pub seat_lock_ttl_secs: i64,
    pub max_tickets_per_order: i32,
    // Payments — SATIM gateway for CIB/DAHABIA [M §7.4]
    pub payment_mode: String, // "mock" | "satim"
    pub satim_base_url: String,
    pub satim_username: String,
    pub satim_password: String,
    pub satim_terminal: String,
    pub return_base_url: String,
    pub bootstrap_token: String, // shared secret to promote the first admin
    // Rate limiting [M §10.3]
    pub rate_window_secs: i64,
    pub rate_max: i64,
    pub login_fail_window_secs: i64,
    pub login_fail_max: i64,
    // Email (SMTP) for notifications [M §7.6] — blank until a provider is set
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub smtp_from: String,
}

impl Config {
    pub fn from_env() -> Self {
        fn var(key: &str, default: &str) -> String {
            std::env::var(key).unwrap_or_else(|_| default.to_string())
        }
        fn num(key: &str, default: i64) -> i64 {
            std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
        }
        Config {
            database_url: var("DATABASE_URL", "postgres://stade:stade@localhost:5432/stade"),
            redis_url: var("REDIS_URL", "redis://localhost:6379"),
            // Honor $PORT (Render/Railway/Fly/Heroku inject it) else BIND_ADDR.
            bind_addr: std::env::var("PORT")
                .map(|p| format!("0.0.0.0:{p}"))
                .unwrap_or_else(|_| var("BIND_ADDR", "0.0.0.0:8080")),
            jwt_secret: var("JWT_SECRET", "change-me-in-production-min-32-bytes-long"),
            jwt_access_ttl_secs: num("JWT_ACCESS_TTL_SECS", 900),
            jwt_refresh_ttl_secs: num("JWT_REFRESH_TTL_SECS", 2_592_000),
            qr_scheme: var("QR_SCHEME", "ed25519"),
            seat_lock_ttl_secs: num("SEAT_LOCK_TTL_SECS", 600),
            max_tickets_per_order: num("MAX_TICKETS_PER_ORDER", 5) as i32,
            // SATIM: real mode only when credentials are present (API not yet
            // provisioned → defaults to "mock" so dev/tests work).
            payment_mode: {
                let explicit = std::env::var("PAYMENT_MODE").ok();
                let has_creds = std::env::var("SATIM_USERNAME").is_ok() && std::env::var("SATIM_PASSWORD").is_ok();
                explicit.unwrap_or_else(|| if has_creds { "satim".into() } else { "mock".into() })
            },
            satim_base_url: var("SATIM_BASE_URL", "https://cib.satim.dz/payment/rest"),
            satim_username: var("SATIM_USERNAME", ""),
            satim_password: var("SATIM_PASSWORD", ""),
            satim_terminal: var("SATIM_TERMINAL", ""),
            return_base_url: var("RETURN_BASE_URL", "http://localhost:8080"),
            bootstrap_token: var("BOOTSTRAP_TOKEN", ""),
            rate_window_secs: num("RATE_WINDOW_SECS", 60),
            rate_max: num("RATE_MAX", 600),            // per key per window
            login_fail_window_secs: num("LOGIN_FAIL_WINDOW_SECS", 900),
            login_fail_max: num("LOGIN_FAIL_MAX", 5),
            smtp_host: var("SMTP_HOST", ""),
            smtp_port: num("SMTP_PORT", 587) as u16,
            smtp_user: var("SMTP_USER", ""),
            smtp_pass: var("SMTP_PASS", ""),
            smtp_from: var("SMTP_FROM", "no-reply@stade-aliammar.dz"),
        }
    }
}
