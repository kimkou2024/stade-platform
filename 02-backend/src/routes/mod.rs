pub mod access;
pub mod account;
pub mod admin;
pub mod auth;
pub mod catalog;
pub mod health;
pub mod invitations;
pub mod purchase;
pub mod reporting;
pub mod zones;

use axum::routing::{get, patch, post};
use axum::Router;

use crate::state::AppState;

/// Full API surface (blueprint §7).
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        // reference seating
        .route("/api/zones", get(zones::list_zones))
        .route("/api/zones/sections", get(zones::list_sections))
        // auth [M §7.1.2, §7.1.3]
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/refresh", post(auth::refresh))
        .route("/api/auth/2fa/setup", post(auth::twofa_setup))
        .route("/api/auth/2fa/verify", post(auth::twofa_verify))
        .route("/api/auth/password/forgot", post(auth::password_forgot))
        .route("/api/auth/password/reset", post(auth::password_reset))
        .route("/api/me", get(auth::me).patch(account::update_me).delete(account::delete_me))
        .route("/api/me/id-document", post(account::set_id_document))
        .route("/api/me/payment-methods", post(account::add_payment_method).get(account::list_payment_methods))
        .route("/api/me/notifications", get(account::notifications))
        // catalog [M §7.1.1, §7.3]
        .route("/api/events", get(catalog::list_events))
        .route("/api/events/:id", get(catalog::get_event))
        .route("/api/events/:id/zones", get(catalog::event_zones))
        // purchase [M §7.1.4]
        .route("/api/events/:id/seat-lock", post(purchase::seat_lock))
        .route("/api/orders", post(purchase::create_order))
        .route("/api/orders/:id/pay", post(purchase::pay_order))
        .route("/api/payments/satim/return", get(purchase::satim_return))
        .route("/api/me/tickets", get(purchase::my_tickets))
        .route("/api/tickets/:id/pdf", get(purchase::ticket_pdf))
        // access control [M §8]
        .route("/api/access/public-key", get(access::public_key))
        .route("/api/access/events/:id/manifest", get(access::manifest))
        .route("/api/access/scan", post(access::scan))
        .route("/api/access/sync", post(access::sync))
        .route("/api/access/invitation-scan", post(invitations::scan))
        // admin [M §7.2, §9.2]
        .route("/api/admin/bootstrap", post(admin::bootstrap))
        .route("/api/admin/events", post(admin::create_event).get(admin::list_events))
        .route("/api/admin/events/:id/zones", post(admin::configure_zone))
        .route("/api/admin/events/:id/status", patch(admin::set_event_status))
        .route("/api/admin/events/:id/invitations", post(invitations::create).get(invitations::list))
        .route("/api/admin/subscriptions", post(admin::create_subscription).get(admin::list_subscriptions))
        .route("/api/admin/audit-log", get(admin::audit_log))
        .route("/api/admin/retention/purge", post(admin::retention_purge))
        // reporting [M §8.5, §9.2, §9.3]
        .route("/api/admin/events/:id/reports/sales", get(reporting::sales_report))
        .route("/api/admin/events/:id/exports/spectators", get(reporting::export_spectators))
        .route("/api/admin/events/:id/dashboard", get(reporting::dashboard))
        .with_state(state)
}
