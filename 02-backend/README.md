# stade-backend

E-ticketing & access-control backend for **Stade Ali Ammar « Ali la Pointe », Douira** (SOGISL).
Phase 1 foundation, built strictly from the tender files (see `BLUEPRINT_technique.md`).

**Imposed stack** `[M §11.1]`: Rust · Axum · Tokio · PostgreSQL · Redis.

## What's in this scaffold
- `migrations/0001_init.sql` — full schema (events, zones, tickets, orders, payments, access logs, RBAC, audit, fraud) — blueprint §5.
- `migrations/0002_seed_zones.sql` — the 8 zones / 14 sections seeded verbatim from the capacity sheet `[X]`, with a self-verifying seat-total check.
- `src/` — Axum app: config, error type, state, Argon2id + JWT auth primitives `[M §7.1.3, §10.2]`, health/readiness, and reference-seating endpoints.
- `docker-compose.yml` — local Postgres 16 + Redis 7.

## Run locally
```bash
cp .env.example .env
docker compose up -d          # Postgres + Redis
cargo run                     # applies migrations on startup, serves :8080
```
Then:
```bash
curl localhost:8080/health
curl localhost:8080/api/zones
curl localhost:8080/api/zones/sections
```

## Verified
- `cargo check` — compiles clean (stub warnings only).
- Both migrations applied against Postgres 16; seed asserts the capacity figures from `[X]`.

## Seat model
Zone-quota counting (not numbered seats), per blueprint §2 #4 — the spec and `[X]` give zone capacities, not a seat map. A `seat` table referencing `tribune_section` can be added later if SOGISL requires numbered seating.

## Capacity note (from `[X]`)
Sections sum to **38,399**; the sheet's printed total is **38,181** (= all minus Presse 218). The seed is faithful to the per-section figures and flags the gap — resolve with SOGISL (blueprint open question #6).

## Next modules (blueprint §11–§12)
auth routes + 2FA · catalog & availability · seat-lock (Redis TTL) · orders (1–5 cap) · payment adapters (CIB/DAHABIA) · ticket issuance + signed QR · PDF generation · access manifest/scan/offline-sync · admin & RBAC · reporting & supervision.
