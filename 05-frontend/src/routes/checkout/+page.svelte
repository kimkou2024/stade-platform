<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';
  import { get } from 'svelte/store';
  import { t, locale, fmtDZD } from '$lib/i18n';
  import { cart, issued } from '$lib/data';
  import { api, isAuthed } from '$lib/api';

  const c = get(cart);
  let pm = 'cib';
  let names: string[] = Array.from({ length: c.qty }, () => '');
  let left = 600;
  let timer: ReturnType<typeof setInterval>;
  let busy = false;
  let error = '';
  $: mm = String(Math.floor(left / 60)).padStart(2, '0');
  $: ss = String(left % 60).padStart(2, '0');
  $: total = c.priceDzd * c.qty;
  $: canPay = c.eventId && c.zoneId; // live purchase needs real backend ids

  onMount(() => { timer = setInterval(() => { if (left > 0) left -= 1; }, 1000); });
  onDestroy(() => clearInterval(timer));

  async function pay() {
    error = '';
    if (!canPay) { error = 'Achat indisponible en mode démo — connectez le backend.'; return; }
    if (!isAuthed()) { if (browser) sessionStorage.setItem('after_login', '/checkout'); goto('/login'); return; }
    busy = true;
    try {
      const holders = names.map((n, i) => n.trim() || `Billet ${i + 1}`);
      const lock = await api.seatLock(c.eventId!, { zone_id: c.zoneId!, qty: c.qty });
      const order = await api.createOrder({ lock_id: lock.lock_id, holder_names: holders });
      const res = await api.pay(order.order_id, { lock_id: lock.lock_id, holder_names: holders, gateway: pm });
      if (res.status === 'redirect') {
        if (browser) window.location.href = res.form_url;   // SATIM hosted page
      } else {
        issued.set(res.tickets);
        goto('/ticket');
      }
    } catch (e: any) { error = e.message; } finally { busy = false; }
  }
</script>

<div class="screen">
  <button class="back" on:click={() => history.back()}>‹ {$t('back')}</button>
  <h2 style="font-size:20px;margin-bottom:12px">{$t('checkout')}</h2>

  <div class="summary">
    <div class="line"><span>{c.eventTitle}</span></div>
    <div class="line"><span>{c.zoneName} × {c.qty}</span><span class="tnum">{fmtDZD(total, $locale)}</span></div>
    <div class="line total"><span>{$t('total')}</span><span class="tnum">{fmtDZD(total, $locale)}</span></div>
  </div>

  <div class="count">⏳ {$t('hold')} <span class="tnum">{mm}:{ss}</span></div>

  <div class="section-title" style="margin-top:4px">Titulaires</div>
  {#each names as _, i}
    <input class="summary" style="display:block;width:100%;padding:10px;margin-bottom:8px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)"
           placeholder={`Nom du titulaire ${i + 1}`} bind:value={names[i]} />
  {/each}

  <div class="section-title" style="margin-top:4px">{$t('paym')}</div>
  <div class="pay">
    <label class:sel={pm === 'cib'}><input type="radio" name="pm" value="cib" bind:group={pm} style="position:absolute;opacity:0" />CIB<span class="scheme">Carte interbancaire</span></label>
    <label class:sel={pm === 'dahabia'}><input type="radio" name="pm" value="dahabia" bind:group={pm} style="position:absolute;opacity:0" />DAHABIA<span class="scheme">Algérie Poste</span></label>
  </div>
  <div class="trust">🔒 {$t('secure')}</div>
  <div class="trust">🛡 {$t('loi')}</div>
  {#if error}<p style="color:var(--danger);margin-top:10px">{error}</p>{/if}
</div>

<div class="actionbar">
  <div class="tot"><div class="l">{$t('total')}</div><div class="v tnum">{fmtDZD(total, $locale)}</div></div>
  <button class="btn" disabled={busy} on:click={pay}>{busy ? '…' : $t('pay')}</button>
</div>
