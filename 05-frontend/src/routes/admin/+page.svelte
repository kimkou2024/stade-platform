<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';
  import { api, isAuthed, type ApiEvent } from '$lib/api';

  let events: ApiEvent[] = [];
  let error = '';
  let loading = true;
  // new-event form
  let title = '';
  let date = '';
  let capacity: number | undefined;
  let creating = false;

  const STATUS_LABEL: Record<string, string> = {
    draft: 'Brouillon', published: 'Publié', sales_open: 'Vente ouverte',
    sales_closed: 'Vente fermée', closed: 'Clôturé', cancelled: 'Annulé', postponed: 'Reporté'
  };

  onMount(load);
  async function load() {
    loading = true; error = '';
    if (!isAuthed()) { if (browser) sessionStorage.setItem('after_login', '/admin'); goto('/login'); return; }
    try { events = (await api.adminEvents()).events; }
    catch (e: any) { error = e.message === 'unauthorized' ? "Accès réservé au personnel. Connectez-vous avec un compte staff." : e.message; }
    loading = false;
  }

  async function create() {
    error = ''; if (!title || !date) { error = 'Titre et date requis.'; return; }
    creating = true;
    try {
      const r = await api.adminCreateEvent({ title, starts_at: new Date(date).toISOString(), capacity });
      goto(`/admin/events/${r.id}`);
    } catch (e: any) { error = e.message; } finally { creating = false; }
  }
</script>

<div class="screen">
  <h2 style="font-size:20px">Back-office</h2>
  <div class="mut" style="font-size:13px;margin-top:2px">Administration des événements — SOGISL</div>
  {#if error}<p style="color:var(--danger)">{error}</p>{/if}

  <div class="section-title">Nouvel événement</div>
  <div class="summary" style="display:grid;gap:10px">
    <input placeholder="Titre (ex: USMA — MCA)" bind:value={title}
           style="padding:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
    <input type="datetime-local" bind:value={date}
           style="padding:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
    <input type="number" placeholder="Capacité (optionnel)" bind:value={capacity}
           style="padding:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
    <button class="btn" disabled={creating} on:click={create}>{creating ? '…' : 'Créer'}</button>
  </div>

  <div class="section-title">Événements</div>
  {#each events as e}
    <a class="match" href={`/admin/events/${e.id}`} style="text-decoration:none">
      <span class="info"><span class="t">{e.title}</span><br /><span class="s">{new Date(e.starts_at).toLocaleString('fr')}</span></span>
      <span class="price"><span class="badge {e.status === 'sales_open' ? 'b-ok' : 'b-low'}">{STATUS_LABEL[e.status] ?? e.status}</span></span>
    </a>
  {:else}
    {#if !loading}<p class="mut">Aucun événement.</p>{/if}
  {/each}
</div>
