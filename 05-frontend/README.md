# stade-web (SvelteKit)

Public site + back-office for Stade Ali Ammar. Imposed stack `[M §11.1]`:
**SvelteKit + TypeScript + Bun + SSR**, consuming the Axum REST API.

> Status: **scaffold, now carrying the full design system.** The Material-inspired,
> marketplace-flow look from the clickable prototype (`09-design-prototype/`) is
> ported here as the real app: shared tokens in `src/app.css`, trilingual + RTL
> `[M §13]`, and the complete buyer flow across routes.

## Run
```bash
bun install          # or: npm install
bun run dev          # http://localhost:5173 (API expected on :8080)
```
Set `PUBLIC_API_BASE` to point at the backend (defaults to `http://localhost:8080`).

## Design system
- `src/app.css` — the single source of visual truth: colour tokens (stadium green
  primary, amber VIP accent, green-biased neutrals), Cairo/Tajawal type (both cover
  Arabic + Latin), Material elevation cards, chips, sticky action bar. Light + dark.
- `src/lib/i18n.ts` — ar/fr/en dictionary + RTL flags + `fmtDZD`; `<html dir>` flips for Arabic.

## Routes (buyer flow)
- `/` — hero next match + upcoming events list.
- `/events/[id]` — interactive stadium map, zone list with live availability, 1–5 stepper, sticky total.
- `/checkout` — order summary, seat-lock countdown, CIB/DAHABIA payment cards, Loi 18-07 + 2FA trust lines.
- `/ticket` — success + QR + Ed25519/offline note.
- `/login` — login with 2FA field.

## Live backend wiring
The buyer flow is wired to the Axum API:
- Home + event pages load from `/api/events` and `/api/events/:id/zones` (real zone
  UUIDs, live availability). If the backend is unreachable, pages fall back to a
  clearly-labelled **demo** mode for display only (purchase disabled).
- Checkout runs the real **seat-lock → order → pay** flow. In **mock** payment mode
  tickets come straight back and render on `/ticket` with their signed-QR token. In
  **SATIM** mode `pay` returns a `form_url`; the app redirects to SATIM's hosted page,
  and `/checkout/return?orderId=…` confirms the payment and shows the issued tickets.
- Login/register store JWT tokens (`src/lib/api.ts`) and resume the checkout.

Point the app at the backend with `PUBLIC_API_BASE` (default `http://localhost:8080`).
Verified: `svelte-check` 0/0 and `vite build` OK. A full click-through needs the
backend running (`cd ../02-backend && cargo run`) alongside `npm run dev`.

## Back-office (admin) — wired
- `/admin` — staff-guarded: create events, list all events (incl. drafts via
  `GET /api/admin/events`), open each to manage.
- `/admin/events/[id]` — per-zone config (quota / price / category against the
  reference zones), sale-status controls (publish / open / close / cancel), and
  live **sales** + **supervision** reports `[M §9.2, §9.3, §8.5]`.
- Staff access needs a staff JWT: register a user, then promote with
  `POST /api/admin/bootstrap` (`X-Bootstrap-Token`), then log in — the token now
  carries `role=admin`.

## User space — wired
- `/me` — profile, **enable 2FA** (shows the otpauth URL + secret, confirms a code) `[M §7.1.2, §7.1.3]`, logout.
- `/me/tickets` — ticket wallet from `/api/me/tickets`, each with its signed-QR `[M §7.7]`.

## Admin — also covers
VIP invitations A/B/C with optional second check `[M §8.4]`, and spectator-list
**CSV export** `[M §9.2]`, on the event page.

## QR + print
- Tickets and the wallet render **real scannable QR** of the signed token (`qrcode` lib, `src/lib/qr.ts`).
- `/admin/events/[id]/invitations/print` lays out **4 VIP invitations per A4** with print CSS `[M §8.4]` — the "Imprimer (4 par A4)" button on the event page opens it.

## Still to build
Push notifications `[M §7.6]` (needs an email/SMS provider — infra).
