// Typed API client for the Axum backend. Token kept in memory + localStorage.
import { browser } from '$app/environment';
import { env } from '$env/dynamic/public';

const BASE = (browser && (window as any).PUBLIC_API_BASE) || env.PUBLIC_API_BASE || 'http://localhost:8080';

function read(k: string): string | null { try { return browser ? localStorage.getItem(k) : null; } catch { return null; } }
function write(k: string, v: string) { try { if (browser) localStorage.setItem(k, v); } catch { /* ignore */ } }
function del(k: string) { try { if (browser) localStorage.removeItem(k); } catch { /* ignore */ } }

let accessToken: string | null = read('access_token');
export function setTokens(access: string, refresh: string) { accessToken = access; write('access_token', access); write('refresh_token', refresh); }
export function clearTokens() { accessToken = null; del('access_token'); del('refresh_token'); }
export function isAuthed(): boolean { return !!accessToken; }

async function req<T>(method: string, path: string, body?: unknown): Promise<T> {
  const headers: Record<string, string> = { 'content-type': 'application/json' };
  if (accessToken) headers['authorization'] = `Bearer ${accessToken}`;
  const res = await fetch(`${BASE}${path}`, { method, headers, body: body ? JSON.stringify(body) : undefined });
  if (!res.ok) {
    let msg = `HTTP ${res.status}`;
    try { msg = (await res.json())?.error ?? msg; } catch { /* keep */ }
    throw new Error(msg);
  }
  return res.json() as Promise<T>;
}

// Fetch a text body (e.g. CSV export) with auth and return it.
async function reqText(path: string): Promise<string> {
  const headers: Record<string, string> = {};
  if (accessToken) headers['authorization'] = `Bearer ${accessToken}`;
  const res = await fetch(`${BASE}${path}`, { headers });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.text();
}

// Fetch a binary body (e.g. PDF) with auth and trigger a download.
export async function downloadAuthed(path: string, filename: string) {
  const headers: Record<string, string> = {};
  if (accessToken) headers['authorization'] = `Bearer ${accessToken}`;
  const res = await fetch(`${BASE}${path}`, { headers });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const url = URL.createObjectURL(await res.blob());
  const a = document.createElement('a'); a.href = url; a.download = filename; a.click();
  URL.revokeObjectURL(url);
}

// --- shapes ---
export interface ApiEvent { id: string; title: string; kind: string; starts_at: string; status: string; }
export interface ApiZone { zone_id: string; code: string; name: string; price_dzd: string; quota: number; issued: number; held: number; available: number; }
export interface Tokens { access_token: string; refresh_token: string; }
export interface IssuedTicket { ticket_id: string; holder: string; qr: string; }
export type PayResult =
  | { status: 'paid'; order_id: string; tickets: IssuedTicket[] }
  | { status: 'redirect'; form_url: string; satim_order_id: string };

export const api = {
  // auth [M §7.1.2, §7.1.3]
  register: (b: { email: string; password: string; full_name: string; locale?: string; consent: boolean }) => req('POST', '/api/auth/register', b),
  passwordForgot: (email: string) => req<{ ok: boolean; dev_token: string | null }>('POST', '/api/auth/password/forgot', { email }),
  passwordReset: (token: string, new_password: string) => req('POST', '/api/auth/password/reset', { token, new_password }),
  login: (b: { email: string; password: string; totp_code?: string }) => req<Tokens>('POST', '/api/auth/login', b),
  me: () => req<{ id: string; email: string; full_name: string; locale: string; role: string }>('GET', '/api/me'),
  // catalog [M §7.1.1, §7.3] (public)
  events: () => req<ApiEvent[]>('GET', '/api/events'),
  event: (id: string) => req<ApiEvent>('GET', `/api/events/${id}`),
  eventZones: (id: string) => req<{ event_id: string; zones: ApiZone[] }>('GET', `/api/events/${id}/zones`),
  // purchase [M §7.1.4]
  seatLock: (eid: string, b: { zone_id: string; qty: number }) => req<{ lock_id: string; locked_until: string }>('POST', `/api/events/${eid}/seat-lock`, b),
  createOrder: (b: { lock_id: string; holder_names: string[] }) => req<{ order_id: string; total_dzd: string }>('POST', '/api/orders', b),
  pay: (oid: string, b: { lock_id: string; holder_names: string[]; gateway?: string }) => req<PayResult>('POST', `/api/orders/${oid}/pay`, b),
  satimReturn: (orderId: string) => req<{ status: string; tickets?: IssuedTicket[] }>('GET', `/api/payments/satim/return?orderId=${encodeURIComponent(orderId)}`),
  myTickets: () => req<{ tickets: IssuedTicket[] }>('GET', '/api/me/tickets'),
  // reference seating [X]
  zonesRef: () => req<Array<{ id: string; code: string; name: string; kind: string; sellable: boolean }>>('GET', '/api/zones'),
  // admin / back-office [M §7.2, §9.2]
  adminEvents: () => req<{ events: ApiEvent[] }>('GET', '/api/admin/events'),
  adminCreateEvent: (b: { title: string; starts_at: string; kind?: string; capacity?: number }) => req<{ id: string }>('POST', '/api/admin/events', b),
  adminConfigureZone: (eid: string, b: { zone_id: string; quota: number; price_dzd: string; ticket_category: string; active?: boolean }) => req('POST', `/api/admin/events/${eid}/zones`, b),
  adminSetStatus: (eid: string, status: string) => req('PATCH', `/api/admin/events/${eid}/status`, { status }),
  adminSales: (eid: string) => req<{ total_sold: number; by_zone: Array<{ zone: string; sold: number; revenue_dzd: string }> }>('GET', `/api/admin/events/${eid}/reports/sales`),
  adminDashboard: (eid: string) => req<{ entries: number; invalid_scans: number; per_gate: Array<{ gate: string; entries: number }> }>('GET', `/api/admin/events/${eid}/dashboard`),
  // 2FA [M §7.1.3]
  twofaSetup: () => req<{ otpauth_url: string; secret: string }>('POST', '/api/auth/2fa/setup'),
  twofaVerify: (code: string) => req<{ twofa_enabled: boolean }>('POST', '/api/auth/2fa/verify', { code }),
  // account [M §7.1.2, §7.7, §10.1]
  updateMe: (b: { full_name?: string; locale?: string }) => req('PATCH', '/api/me', b),
  deleteMe: () => req('DELETE', '/api/me'),
  setIdDocument: (reference: string) => req('POST', '/api/me/id-document', { reference }),
  addPaymentMethod: (b: { scheme: string; gateway_token: string; last4?: string; exp?: string }) => req('POST', '/api/me/payment-methods', b),
  listPaymentMethods: () => req<{ methods: Array<{ id: string; scheme: string; last4: string | null }> }>('GET', '/api/me/payment-methods'),
  notifications: () => req<{ notifications: Array<{ channel: string; subject: string; status: string; at: string }> }>('GET', '/api/me/notifications'),
  // VIP invitations [M §8.4]
  adminCreateInvitation: (eid: string, b: { tribune: string; holder_name: string; second_check_required?: boolean }) => req<{ id: string; tribune: string; qr: string }>('POST', `/api/admin/events/${eid}/invitations`, b),
  adminInvitations: (eid: string) => req<{ invitations: Array<{ id: string; tribune: string; holder: string; status: string; qr: string }> }>('GET', `/api/admin/events/${eid}/invitations`),
  // spectator CSV export [M §9.2]
  exportSpectatorsCsv: (eid: string) => reqText(`/api/admin/events/${eid}/exports/spectators`)
};
