<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';
  import { api, isAuthed } from '$lib/api';
  import { qrDataUrl } from '$lib/qr';

  $: eid = $page.params.id ?? '';
  let items: Array<{ id: string; tribune: string; holder: string; qr: string; qrImg?: string }> = [];
  let title = '';
  let error = '';

  onMount(async () => {
    if (!isAuthed()) { if (browser) sessionStorage.setItem('after_login', `/admin/events/${eid}/invitations/print`); goto('/login'); return; }
    try {
      title = (await api.event(eid)).title;
      items = (await api.adminInvitations(eid)).invitations;
      for (const it of items) it.qrImg = await qrDataUrl(it.qr, 150);
      items = items;
    } catch (e: any) { error = e.message; }
  });
  function print() { if (browser) window.print(); }
</script>

<!-- Disposition: 4 invitations par feuille A4 [M §8.4] -->
<div class="toolbar">
  <button class="back" on:click={() => goto(`/admin/events/${eid}`)}>‹ Retour</button>
  <strong>Invitations VIP — {title}</strong>
  <button class="btn" on:click={print}>Imprimer</button>
</div>
{#if error}<p style="color:var(--danger);padding:0 16px">{error}</p>{/if}

<div class="sheet">
  {#each items as it}
    <div class="inv inv-{it.tribune}">
      <div class="inv-head">
        <span class="crest">SA</span>
        <div><div class="club">Stade Ali Ammar</div><div class="sub">Invitation VIP · Tribune {it.tribune}</div></div>
      </div>
      <div class="inv-body">
        <div class="ev">{title}</div>
        <div class="guest">{it.holder}</div>
        {#if it.qrImg}<img src={it.qrImg} alt="QR invitation" width="150" height="150" />{/if}
        <div class="note">Présentez ce QR à l'entrée VIP. Vérification hors ligne (Ed25519).</div>
      </div>
    </div>
  {:else}
    <p class="mut" style="padding:16px">Aucune invitation. Créez-en depuis la page de l'événement.</p>
  {/each}
</div>

<style>
  .toolbar { display:flex; align-items:center; justify-content:space-between; gap:12px; padding:12px 16px; position:sticky; top:0; background:var(--surface); border-bottom:1px solid var(--line); z-index:5; }
  .toolbar .back { margin:0; }
  /* 2 columns × 2 rows = 4 per A4 page */
  .sheet { display:grid; grid-template-columns:1fr 1fr; gap:10mm; padding:12mm; max-width:210mm; margin:0 auto; }
  .inv { border:1.5px solid var(--line); border-radius:10px; overflow:hidden; background:#fff; color:#141a16; min-height:120mm; display:flex; flex-direction:column; break-inside:avoid; }
  .inv-head { display:flex; align-items:center; gap:10px; padding:12px 14px; color:#fff; background:linear-gradient(160deg,#0d7a3b,#0a5e2e); }
  .inv-A .inv-head { background:linear-gradient(160deg,#0d7a3b,#0a5e2e); }
  .inv-B .inv-head { background:linear-gradient(160deg,#c98a12,#9c6c0d); }
  .inv-C .inv-head { background:linear-gradient(160deg,#8a1538,#5e0e26); }
  .crest { width:30px;height:30px;border-radius:8px;background:#fff;color:#0a5e2e;display:grid;place-items:center;font-weight:800;font-family:"Cairo",sans-serif }
  .club { font-family:"Cairo",sans-serif; font-weight:800; }
  .sub { font-size:12px; opacity:.9; }
  .inv-body { flex:1; display:flex; flex-direction:column; align-items:center; justify-content:center; gap:10px; padding:16px; text-align:center; }
  .ev { font-family:"Cairo",sans-serif; font-weight:800; font-size:16px; }
  .guest { font-size:14px; }
  .note { font-size:10px; color:#5d6b61; }
  @media print {
    :global(.bar) { display:none !important; }
    .toolbar { display:none !important; }
    .sheet { padding:0; gap:0; }
    .inv { border-color:#999; border-radius:0; margin:0; }
    @page { size:A4; margin:10mm; }
  }
</style>
