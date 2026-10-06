// Demo catalog data for the scaffold. In production these come from the API
// (GET /api/events, /api/events/:id/zones). Capacities from the fiche technique [X].
import { writable } from 'svelte/store';
import type { Locale } from './i18n';

export interface Match { id: number; h: string; a: string; hn: string; an: string; comp: string; d: string; m: string; date: string; from: number; }
export interface Zone { code: string; name: Record<Locale, string>; gate: string; price: number; avail: number; cat: 'ok' | 'low' | 'vip'; }

export const MATCHES: Match[] = [
  { id: 0, h: 'USMA', a: 'MCA', hn: 'USM Alger', an: 'MC Alger', comp: 'Ligue 1', d: '01', m: 'Nov', date: 'Sam 01 Nov · 18:00', from: 500 },
  { id: 1, h: 'CRB', a: 'JSK', hn: 'CR Belouizdad', an: 'JS Kabylie', comp: 'Ligue 1', d: '08', m: 'Nov', date: 'Sam 08 Nov · 20:00', from: 500 },
  { id: 2, h: 'ESS', a: 'CSC', hn: 'ES Sétif', an: 'CS Constantine', comp: 'Ligue 1', d: '15', m: 'Nov', date: 'Sam 15 Nov · 17:00', from: 600 },
  { id: 3, h: 'DZ', a: 'TUN', hn: 'Algérie', an: 'Tunisie', comp: 'Amical', d: '20', m: 'Nov', date: 'Jeu 20 Nov · 20:00', from: 800 }
];

export const ZONES: Zone[] = [
  { code: 'Z1', name: { fr: 'Zone 1 — Sud', ar: 'المنطقة 1 — الجنوب', en: 'Zone 1 — South' }, gate: 'V19', price: 500, avail: 9959, cat: 'ok' },
  { code: 'Z2E', name: { fr: 'Zone 2 — Centre', ar: 'المنطقة 2 — الوسط', en: 'Zone 2 — Centre' }, gate: 'V11', price: 800, avail: 1040, cat: 'low' },
  { code: 'Z3', name: { fr: 'Zone 3 — Nord', ar: 'المنطقة 3 — الشمال', en: 'Zone 3 — North' }, gate: 'V04', price: 500, avail: 9686, cat: 'ok' },
  { code: 'VIP', name: { fr: 'VIP — Mezghena', ar: 'في آي بي — مزغنة', en: 'VIP — Mezghena' }, gate: 'VIP', price: 3000, avail: 88, cat: 'vip' },
  { code: 'VVIP', name: { fr: 'VVIP — Doyen', ar: 'في في آي بي — العميد', en: 'VVIP — Doyen' }, gate: 'VVIP', price: 6000, avail: 21, cat: 'vip' }
];

import type { IssuedTicket } from './api';

// Purchase state shared across the live flow (seat-lock → order → pay → ticket).
export const cart = writable<{
  eventId: string | null;       // backend UUID
  eventTitle: string;
  zoneId: string | null;        // backend UUID
  zoneName: string;
  gate: string;
  qty: number;
  priceDzd: number;
}>({ eventId: null, eventTitle: '', zoneId: null, zoneName: '', gate: '', qty: 1, priceDzd: 0 });

// Tickets issued by the backend after payment (real signed-QR tokens).
export const issued = writable<IssuedTicket[]>([]);
