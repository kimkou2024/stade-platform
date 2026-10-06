<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { t, locale, fmtDZD } from '$lib/i18n';
  import { MATCHES } from '$lib/data';
  import { api, type ApiEvent } from '$lib/api';

  let events: ApiEvent[] = [];
  let live = false;
  let loading = true;

  onMount(async () => {
    try { events = await api.events(); live = events.length > 0; }
    catch { live = false; }
    loading = false;
  });

  function openLive(e: ApiEvent) { goto(`/events/${e.id}`); }
  function openDemo(id: number) { goto(`/events/demo-${id}`); }
</script>

<div class="screen">
  {#if !loading && !live}
    <div class="count" style="background:color-mix(in srgb,var(--amber) 14%,transparent)">
      Mode démo — backend non connecté. Les événements ci-dessous sont des exemples.
    </div>
  {/if}

  <div class="section-title">{$t('upcoming')}</div>

  {#if live}
    {#each events as e}
      <button class="match" on:click={() => openLive(e)}>
        <span class="date"><span class="d tnum">{new Date(e.starts_at).getDate()}</span><br /><span class="m">{new Date(e.starts_at).toLocaleString($locale, { month: 'short' })}</span></span>
        <span class="info"><span class="t">{e.title}</span><br /><span class="s">{new Date(e.starts_at).toLocaleString($locale)}</span></span>
        <span class="price"><span class="v">→</span></span>
      </button>
    {/each}
  {:else}
    {#each MATCHES as m}
      <button class="match" on:click={() => openDemo(m.id)}>
        <span class="date"><span class="d tnum">{m.d}</span><br /><span class="m">{m.m}</span></span>
        <span class="info"><span class="t">{m.hn} — {m.an}</span><br /><span class="s">{m.comp} · {m.date}</span></span>
        <span class="price"><span class="v tnum">{fmtDZD(m.from, $locale)}</span></span>
      </button>
    {/each}
  {/if}
  <div class="foot">SOGISL · Stade Ali Ammar « Ali la Pointe », Douira · 38 181 places</div>
</div>
