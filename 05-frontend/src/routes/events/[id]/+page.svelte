<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { t, locale, fmtDZD } from '$lib/i18n';
  import { MATCHES, ZONES, cart } from '$lib/data';
  import { api, type ApiZone } from '$lib/api';

  $: rawId = $page.params.id ?? '';
  $: demo = rawId.startsWith('demo-');

  let title = '';
  let zones: ApiZone[] = [];
  let selId: string | null = null;
  let qty = 1;
  let error = '';
  let loading = true;

  $: sel = zones.find((z) => z.zone_id === selId) ?? null;
  $: total = sel ? Number(sel.price_dzd) * qty : 0;

  onMount(load);
  async function load() {
    loading = true; error = '';
    if (demo) {
      const m = MATCHES.find((x) => `demo-${x.id}` === rawId) ?? MATCHES[0];
      title = `${m.hn} — ${m.an}`;
      zones = ZONES.map((z) => ({ zone_id: z.code, code: z.code, name: z.name[$locale], price_dzd: String(z.price), quota: z.avail, issued: 0, held: 0, available: z.avail }));
    } else {
      try {
        const ev = await api.event(rawId); title = ev.title;
        zones = (await api.eventZones(rawId)).zones;
      } catch (e: any) { error = e.message; }
    }
    loading = false;
  }

  const MAP = [
    { c: 'Z3', x: 96, y: 12, w: 128, h: 40, fill: '#3b82f6', label: 'Nord' },
    { c: 'Z2E', x: 236, y: 60, w: 40, h: 90, fill: '#22a06b', label: 'Centre' },
    { c: 'Z1', x: 96, y: 158, w: 128, h: 40, fill: '#3b82f6', label: 'Sud' },
    { c: 'VIP', x: 44, y: 60, w: 40, h: 60, fill: '#c98a12', label: 'VIP' },
    { c: 'VVIP', x: 44, y: 124, w: 40, h: 26, fill: '#8a1538', label: 'VVIP' }
  ];
  function selectByCode(code: string) { const z = zones.find((x) => x.code === code); if (z) selId = z.zone_id; }

  function cont() {
    if (!sel) return;
    cart.set({
      eventId: demo ? null : rawId, eventTitle: title,
      zoneId: demo ? null : sel.zone_id, zoneName: sel.name, gate: '',
      qty, priceDzd: Number(sel.price_dzd)
    });
    goto('/checkout');
  }
</script>

<div class="screen">
  <button class="back" on:click={() => goto('/')}>‹ {$t('back')}</button>
  <h2 style="font-size:20px">{title || '…'}</h2>
  {#if demo}<div class="mut" style="font-size:12px;margin-top:4px">Exemple (mode démo)</div>{/if}
  {#if error}<p style="color:var(--danger)">{error}</p>{/if}

  <div class="mapwrap">
    <svg viewBox="0 0 320 210" width="100%" role="img" aria-label="Plan du stade">
      <rect x="110" y="78" width="100" height="54" rx="6" fill="color-mix(in srgb,var(--turf) 25%,transparent)" stroke="var(--turf)" />
      {#each MAP as z}
        {@const zn = zones.find((x) => x.code === z.c)}
        <rect class="zone" class:sel={sel && sel.code === z.c} x={z.x} y={z.y} width={z.w} height={z.h} rx="8"
              fill={z.fill} opacity={zn ? 0.88 : 0.35} role="button" tabindex="0" aria-label={z.label}
              on:click={() => selectByCode(z.c)} on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && selectByCode(z.c)} />
      {/each}
    </svg>
  </div>

  <div class="section-title">{$t('choose')}</div>
  {#each zones as z}
    <div class="zrow" class:sel={selId === z.zone_id} role="button" tabindex="0"
         on:click={() => (selId = z.zone_id)} on:keydown={(e) => e.key === 'Enter' && (selId = z.zone_id)}>
      <span class="nm">
        <span class="z">{z.name}</span><br />
        <span class="badge {z.available > 200 ? 'b-ok' : 'b-low'}">{z.available.toLocaleString('fr')} {$t('available')}</span>
      </span>
      <span class="pr tnum">{fmtDZD(Number(z.price_dzd), $locale)}</span>
    </div>
  {:else}
    {#if !loading}<p class="mut">—</p>{/if}
  {/each}

  <div style="display:flex;align-items:center;justify-content:space-between;margin:14px 2px 2px">
    <span style="font-weight:700;font-family:var(--display)">{$t('qty')}</span>
    <div class="stepper">
      <button on:click={() => (qty = Math.max(1, qty - 1))}>−</button>
      <span class="n tnum">{qty}</span>
      <button on:click={() => (qty = Math.min(5, qty + 1))}>+</button>
    </div>
  </div>
  <div class="mut" style="font-size:12px;margin:6px 2px">{$t('qtyhint')}</div>
</div>

<div class="actionbar">
  <div class="tot"><div class="l">{$t('total')}</div><div class="v tnum">{fmtDZD(total, $locale)}</div></div>
  <button class="btn" disabled={!sel} on:click={cont}>{$t('continue')}</button>
</div>
