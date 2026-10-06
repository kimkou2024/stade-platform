//! Notifications [M §7.6]: email / SMS / push. A real SMTP adapter sends email
//! when SMTP_* is configured; otherwise (blank credentials) it records to
//! `notification_log` and logs — so the flow works end-to-end now and goes live
//! the moment a provider is set. SMS/push plug in the same way. Trilingual
//! bodies are the caller's responsibility [M §13].

use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use uuid::Uuid;

use crate::state::AppState;

/// Record and deliver a notification. Best-effort — never fails the request.
pub async fn send(state: &AppState, user_id: Option<Uuid>, channel: &str, subject: &str, body: &str) {
    tracing::info!("notify[{channel}] to {:?}: {subject}", user_id);

    // Try real email when SMTP is configured and this is an email notification.
    let mut status = "sent";
    if channel == "email" && !state.cfg.smtp_host.is_empty() {
        if let Some(uid) = user_id {
            let to: Option<String> = sqlx::query_scalar("SELECT email FROM user_account WHERE id=$1")
                .bind(uid).fetch_optional(&state.db).await.ok().flatten();
            if let Some(addr) = to {
                status = match deliver_email(&state.cfg, &addr, subject, body).await {
                    Ok(_) => "sent",
                    Err(e) => { tracing::warn!("smtp send failed: {e}"); "failed" }
                };
            }
        }
    }

    let _ = sqlx::query(
        "INSERT INTO notification_log (user_id, channel, subject, body, status) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(user_id).bind(channel).bind(subject).bind(body).bind(status)
    .execute(&state.db).await;
}

async fn deliver_email(cfg: &crate::config::Config, to: &str, subject: &str, body: &str) -> Result<(), String> {
    let email = Message::builder()
        .from(cfg.smtp_from.parse().map_err(|e| format!("from: {e}"))?)
        .to(to.parse().map_err(|e| format!("to: {e}"))?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| format!("build: {e}"))?;
    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp_host)
        .map_err(|e| format!("relay: {e}"))?
        .port(cfg.smtp_port);
    if !cfg.smtp_user.is_empty() {
        builder = builder.credentials(Credentials::new(cfg.smtp_user.clone(), cfg.smtp_pass.clone()));
    }
    let mailer = builder.build();
    mailer.send(email).await.map_err(|e| format!("send: {e}"))?;
    Ok(())
}
