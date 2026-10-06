<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api, downloadAuthed, type ApiEvent } from '$lib/api';

  $: eid = $page.params.id ?? '';
  let ev: ApiEvent | null = null;
  let zonesRef: Array<{ id: string; code: string; name: string; sellable: boolean }> = [];
  let configured: Record<string, { quota: number; price: string }> = {};
  let sales: { total_sold: number; by_zone: Array<{ zone: string; sold: number; revenue_dzd: string }> } | null = null;
  let dash: { entries: number; invalid_scans: number; per_gate: Array<{ gate: string; entries: number }> } | null = null;
  let error = '', notice = '';

  const CATEGORIES = ['local', 'visitor', 'vip', 'vvip', 'official'];
  // per-zone form state
  let form: Record<string, { quota: number; price: string; category: string }> = {};
  // VIP invitations [M §8.4]
  let invitations: Array<{ id: string; tribune: string; holder: string; status: string }> = [];
  let invForm = { tribune: 'A', holder: '', second: false };
  let lastInvQr = '';

  onMount(load);
  async function load() {
    error = '';
    try {
      ev = await api.event(eid);
      zonesRef = (await api.zonesRef()).filter((z) => z.sellable);
      const z = (await api.eventZones(eid)).zones;
      configured = Object.fromEntries(z.map((x) => [x.zone_id, { quota: x.quota, price: x.price_dzd }]));
      for (const zr of zonesRef) {
        const c = configured[zr.id];
        form[zr.id] = { quota: c?.quota ?? 0, price: c?.price ?? '0', category: defaultCat(zr.code) };
      }
      await refreshReports();
      await loadInvitations();
    } catch (e: any) { error = e.message; }
  }
  async function loadInvitations() { try { invitations = (await api.adminInvitations(eid)).invitations; } catch { /* none yet */ } }
  async function createInvitation() {
    error = ''; notice = '';
    try {
      const r = await api.adminCreateInvitation(eid, { tribune: invForm.tribune, holder_name: invForm.holder, second_check_required: invForm.second });
      lastInvQr = r.qr; invForm.holder = ''; notice = 'Invitation créée.'; await loadInvitations();
    } catch (e: any) { error = e.message; }
  }
  async function exportCsv() {
    error = '';
    try {
      const csv = await api.exportSpectatorsCsv(eid);
      const url = URL.createObjectURL(new Blob([csv], { type: 'text/csv' }));
      const a = document.createElement('a'); a.href = url; a.download = `spectators-${eid}.csv`; a.click();
      URL.revokeObjectURL(url);
    } catch (e: any) { error = e.message; }
  }
  function defaultCat(code: string) {
    if (code === 'VIP') return 'vip'; if (code === 'VVIP') return 'vvip';
    if (code === 'OFF') return 'official'; return 'local';
  }
  async function refreshReports() {
    try { sales = await api.adminSales(eid); dash = await api.adminDashboard(eid); } catch { /* ignore until data */ }
  }

  async function saveZone(zoneId: string) {
    error = ''; notice = '';
    try {
      const f = form[zoneId];
      await api.adminConfigureZone(eid, { zone_id: zoneId, quota: Number(f.quota), price_dzd: String(f.price), ticket_category: f.category, active: true });
      notice = 'Zone enregistrée.'; await load();
    } catch (e: any) { error = e.message; }
  }
  async function setStatus(status: string) {
    error = ''; notice = '';
    try { await api.adminSetStatus(eid, status); ev = await api.event(eid); notice = `Statut: ${status}.`; }
    catch (e: any) { error = e.message; }
  }
</script>

<div class="screen">
  <button class="back" on:click={() => goto('/admin')}>‹ Événements</button>
  <h2 style="font-size:20px">{ev?.title ?? '…'}</h2>
  {#if ev}<div class="mut" style="font-size:13px">{new Date(ev.starts_at).toLocaleString('fr')} · statut {ev.status}</div>{/if}
  {#if error}<p style="color:var(--danger)">{error}</p>{/if}
  {#if notice}<p style="color:var(--ok)">{notice}</p>{/if}

  <div class="section-title">Statut de vente</div>
  <div style="display:flex;gap:8px;flex-wrap:wrap">
    <button class="btn" style="padding:8px 12px" on:click={() => setStatus('published')}>Publier</button>
    <button class="btn" style="padding:8px 12px" on:click={() => setStatus('sales_open')}>Ouvrir la vente</button>
    <button class="btn" style="padding:8px 12px;background:var(--mut)" on:click={() => setStatus('sales_closed')}>Fermer</button>
    <button class="btn" style="padding:8px 12px;background:var(--danger)" on:click={() => setStatus('cancelled')}>Annuler</button>
  </div>

  <div class="section-title">Zones (quota, tarif, catégorie)</div>
  {#each zonesRef as z}
    <div class="zrow" style="flex-wrap:wrap;gap:8px">
      <span class="nm" style="min-width:120px"><span class="z">{z.name}</span>
        {#if configured[z.id]}<br /><span class="badge b-ok">configurée · {configured[z.id].quota}</span>{/if}
      </span>
      <input type="number" bind:value={form[z.id].quota} placeholder="quota" style="width:80px;padding:7px;border:1px solid var(--line);border-radius:8px;background:var(--surface);color:var(--fg)" />
      <input type="number" bind:value={form[z.id].price} placeholder="DZD" style="width:90px;padding:7px;border:1px solid var(--line);border-radius:8px;background:var(--surface);color:var(--fg)" />
      <select bind:value={form[z.id].category} style="padding:7px;border:1px solid var(--line);border-radius:8px;background:var(--surface);color:var(--fg)">
        {#each CATEGORIES as c}<option value={c}>{c}</option>{/each}
      </select>
      <button class="btn" style="padding:7px 12px" on:click={() => saveZone(z.id)}>OK</button>
    </div>
  {/each}

  <div class="section-title">Invitations VIP (tribunes A/B/C)</div>
  <div class="zrow" style="flex-wrap:wrap;gap:8px">
    <select bind:value={invForm.tribune} style="padding:7px;border:1px solid var(--line);border-radius:8px;background:var(--surface);color:var(--fg)">
      <option value="A">Tribune A</option><option value="B">Tribune B</option><option value="C">Tribune C</option>
    </select>
    <input bind:value={invForm.holder} placeholder="Nom de l'invité" style="flex:1;min-width:120px;padding:7px;border:1px solid var(--line);border-radius:8px;background:var(--surface);color:var(--fg)" />
    <label style="font-size:12px;display:flex;align-items:center;gap:4px"><input type="checkbox" bind:checked={invForm.second} /> 2ᵉ contrôle</label>
    <button class="btn" style="padding:7px 12px" on:click={createInvitation}>Créer</button>
  </div>
  {#if lastInvQr}<div class="mut" style="font-size:11px;word-break:break-all;margin:6px 2px">QR émis : {lastInvQr.slice(0, 48)}…</div>{/if}
  {#each invitations as inv}
    <div class="line"><span>Tribune {inv.tribune} — {inv.holder}</span><span class="badge {inv.status === 'valid' ? 'b-ok' : 'b-low'}">{inv.status}</span></div>
  {/each}
  {#if invitations.length}
    <a class="btn block" style="margin-top:8px;text-align:center" href={`/admin/events/${eid}/invitations/print`}>Imprimer (4 par A4)</a>
  {/if}

  <div class="section-title">Ventes</div>
  {#if sales}
    <div class="summary">
      <div class="line total"><span>Total vendu</span><span class="tnum">{sales.total_sold}</span></div>
      {#each sales.by_zone as r}<div class="line"><span>{r.zone}</span><span class="tnum">{r.sold} · {r.revenue_dzd} DZD</span></div>{/each}
    </div>
    <div style="display:flex;gap:8px;margin-top:10px">
      <button class="btn" style="flex:1" on:click={exportCsv}>Export CSV</button>
      <button class="btn" style="flex:1" on:click={() => downloadAuthed(`/api/admin/events/${eid}/exports/spectators?format=pdf`, 'spectators.pdf')}>Export PDF</button>
    </div>
  {:else}<p class="mut">—</p>{/if}

  <div class="section-title">Supervision (jour de match)</div>
  {#if dash}
    <div class="summary">
      <div class="line"><span>Entrées</span><span class="tnum">{dash.entries}</span></div>
      <div class="line"><span>Scans invalides</span><span class="tnum">{dash.invalid_scans}</span></div>
      {#each dash.per_gate as g}<div class="line"><span>Porte {g.gate}</span><span class="tnum">{g.entries}</span></div>{/each}
    </div>
  {:else}<p class="mut">—</p>{/if}
</div>
