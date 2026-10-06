<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { get } from 'svelte/store';
  import { t } from '$lib/i18n';
  import { cart, issued } from '$lib/data';
  import { qrDataUrl } from '$lib/qr';

  const tickets = get(issued);
  const c = get(cart);
  let qrs: string[] = [];
  onMount(async () => { qrs = await Promise.all(tickets.map((tk) => qrDataUrl(tk.qr))); });
</script>

<div class="screen">
  <div style="display:flex;align-items:center;gap:10px;color:var(--ok);font-weight:800;font-family:var(--display);margin-bottom:12px">
    <span style="width:30px;height:30px;border-radius:50%;background:var(--ok);color:#fff;display:grid;place-items:center">✓</span>
    {$t('paid')}
  </div>

  {#if tickets.length === 0}
    <p class="mut">Aucun billet à afficher. <button class="back" style="margin:0" on:click={() => goto('/')}>Accueil</button></p>
  {/if}

  {#each tickets as tk, i}
    <div class="ticket" style="margin-bottom:14px">
      <div class="top">
        <div style="font-size:12px;opacity:.85;letter-spacing:.1em;text-transform:uppercase">Billet électronique</div>
        <div style="font-family:var(--display);font-weight:800;font-size:18px;margin-top:4px">{c.eventTitle}</div>
      </div>
      <div class="qrwrap">{#if qrs[i]}<img src={qrs[i]} alt="QR billet" width="180" height="180" style="border-radius:8px" />{/if}</div>
      <div class="perf"></div>
      <div style="padding:14px 18px 18px">
        <div class="kv"><span class="k">{$t('holder')}</span><span>{tk.holder}</span></div>
        <div class="kv"><span class="k">{$t('zone')}</span><span>{c.zoneName}</span></div>
        <div class="kv"><span class="k">ID</span><span class="tnum">{tk.ticket_id.slice(0, 8)}…</span></div>
        <div class="kv"><span class="k">Signature</span><span>Ed25519 ✓</span></div>
      </div>
    </div>
  {/each}

  <button class="btn block" on:click={() => goto('/')}>{$t('done')}</button>
  <div class="foot">{$t('offline')}</div>
</div>
