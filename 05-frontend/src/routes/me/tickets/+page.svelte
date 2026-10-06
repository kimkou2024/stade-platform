<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';
  import { t } from '$lib/i18n';
  import { api, isAuthed, downloadAuthed, type IssuedTicket } from '$lib/api';
  import { qrDataUrl } from '$lib/qr';

  let tickets: IssuedTicket[] = [];
  let qrs: string[] = [];
  let error = '', loading = true;

  onMount(async () => {
    if (!isAuthed()) { if (browser) sessionStorage.setItem('after_login', '/me/tickets'); goto('/login'); return; }
    try {
      tickets = (await api.myTickets()).tickets;
      qrs = await Promise.all(tickets.map((tk) => qrDataUrl(tk.qr)));
    } catch (e: any) { error = e.message; }
    loading = false;
  });
</script>

<div class="screen">
  <button class="back" on:click={() => goto('/me')}>‹ Mon espace</button>
  <h2 style="font-size:20px">Mes billets</h2>
  {#if error}<p style="color:var(--danger)">{error}</p>{/if}
  {#if !loading && tickets.length === 0}<p class="mut">Aucun billet pour le moment.</p>{/if}

  {#each tickets as tk, i}
    <div class="ticket" style="margin-top:12px">
      <div class="qrwrap">{#if qrs[i]}<img src={qrs[i]} alt="QR billet" width="180" height="180" style="border-radius:8px" />{/if}</div>
      <div class="perf"></div>
      <div style="padding:14px 18px 18px">
        <div class="kv"><span class="k">{$t('holder')}</span><span>{tk.holder}</span></div>
        <div class="kv"><span class="k">ID</span><span class="tnum">{tk.ticket_id.slice(0, 8)}…</span></div>
        <div class="kv"><span class="k">Signature</span><span>Ed25519 ✓</span></div>
        <button class="btn block" style="margin-top:10px" on:click={() => downloadAuthed(`/api/tickets/${tk.ticket_id}/pdf`, 'billet.pdf')}>Télécharger le PDF</button>
      </div>
    </div>
  {/each}
  <div class="foot">{$t('offline')}</div>
</div>
