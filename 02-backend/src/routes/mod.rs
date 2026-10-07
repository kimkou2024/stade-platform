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
        // public privacy policy (Google Play requirement)
        .route("/privacy", get(privacy))
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


/// Public privacy policy page for the access app [Google Play].
async fn privacy() -> axum::response::Html<&'static str> {
    axum::response::Html(PRIVACY_HTML)
}

const PRIVACY_HTML: &str = r###"<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Confidentialité StadeDz</title>
<meta name="description" content="Politique de confidentialité de l'application de contrôle d'accès StadeDz (Stade Ali Ammar, Douira) éditée par SOGISL.">
<style>
  :root{
    --bg:#f6f7f9; --card:#ffffff; --ink:#15181d; --muted:#5b636e;
    --line:#e5e8ec; --accent:#1f6f43; --accent-ink:#0f5a35;
  }
  :root:not([data-theme="light"]){ @media (prefers-color-scheme: dark){
    :root{ --bg:#101317; --card:#171b21; --ink:#e9edf2; --muted:#9aa4b0; --line:#262c34; --accent:#49b37d; --accent-ink:#8fe0b6; }
  }}
  :root[data-theme="dark"]{ --bg:#101317; --card:#171b21; --ink:#e9edf2; --muted:#9aa4b0; --line:#262c34; --accent:#49b37d; --accent-ink:#8fe0b6; }
  *{box-sizing:border-box}
  body{margin:0;background:var(--bg);color:var(--ink);
    font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Arial,sans-serif;
    line-height:1.65;font-size:16px}
  .wrap{max-width:760px;margin:0 auto;padding:40px 16px 80px}
  .card{background:var(--card);border:1px solid var(--line);border-radius:14px;padding:28px 26px}
  h1{font-size:26px;margin:0 0 6px;letter-spacing:-.01em}
  .muted{color:var(--muted);font-size:14px;margin:0 0 8px}
  h2{font-size:18px;margin:26px 0 6px;color:var(--accent-ink)}
  a{color:var(--accent)}
  ul{padding-left:20px}
  li{margin:6px 0}
  code{background:var(--bg);border:1px solid var(--line);border-radius:5px;padding:1px 6px;font-size:.9em}
  hr{border:none;border-top:1px solid var(--line);margin:28px 0}
</style>
</head>
<body>
<div class="wrap">
  <div class="card">
    <h1>Politique de confidentialité — StadeDz</h1>
    <p class="muted">Application de contrôle d'accès &amp; billetterie du Stade Ali Ammar (Douira, Algérie), éditée par SOGISL.</p>
    <p class="muted">Dernière mise à jour : 7 octobre 2026</p>

    <h2>1. Éditeur</h2>
    <p>L'application StadeDz (package <code>dz.sogisl.stade.access</code>) est éditée et exploitée par <b>SOGISL</b>. Contact : <a href="mailto:kfoughali@dzlaws.org">kfoughali@dzlaws.org</a>.</p>

    <h2>2. Objet de l'application</h2>
    <p>StadeDz est un outil professionnel destiné au personnel de contrôle d'accès du stade. Il permet de scanner les billets électroniques (QR codes signés) aux portes d'entrée et de vérifier leur validité, y compris hors ligne.</p>

    <h2>3. Données traitées</h2>
    <p>L'application traite les données strictement nécessaires au contrôle d'accès :</p>
    <ul>
      <li><b>Caméra</b> — utilisée uniquement pour lire les QR codes des billets. Les images sont traitées sur l'appareil en temps réel et ne sont <b>jamais</b> enregistrées ni transmises.</li>
      <li><b>Données de scan opérationnelles</b> — identifiant du billet, zone/porte, modèle de l'appareil et horodatage, transmis au serveur de SOGISL pour enregistrer les entrées et éviter la réutilisation des billets.</li>
      <li><b>Jeton d'authentification du personnel</b> — utilisé pour autoriser l'agent à télécharger la liste des billets de l'événement.</li>
    </ul>
    <p>L'application <b>ne collecte pas</b> : localisation, contacts, données de santé, informations financières, ni données publicitaires. Aucun profil du spectateur n'est constitué par l'application.</p>

    <h2>4. Finalité</h2>
    <p>Ces données sont utilisées uniquement pour la <b>fonctionnalité de l'application</b> (contrôle d'accès et sécurité de l'événement), jamais à des fins publicitaires ni de suivi.</p>

    <h2>5. Partage</h2>
    <p>Les données ne sont <b>ni vendues ni partagées</b> avec des tiers. Elles transitent uniquement entre l'application et le serveur de SOGISL.</p>

    <h2>6. Sécurité</h2>
    <p>Toutes les communications réseau sont chiffrées en transit (HTTPS/TLS). Les billets sont validés par signature cryptographique Ed25519.</p>

    <h2>7. Conservation et suppression</h2>
    <p>Les données de scan sont conservées le temps nécessaire à l'exploitation de l'événement. Pour toute demande d'accès ou de suppression, écrivez à <a href="mailto:kfoughali@dzlaws.org">kfoughali@dzlaws.org</a>.</p>

    <h2>8. Modifications</h2>
    <p>Cette politique peut être mise à jour ; la date de dernière mise à jour figure en haut de cette page.</p>

    <hr>
    <p class="muted">© 2026 SOGISL — Stade Ali Ammar, Douira. Contact : <a href="mailto:kfoughali@dzlaws.org">kfoughali@dzlaws.org</a></p>
  </div>
</div>
</body>
</html>
"###;
