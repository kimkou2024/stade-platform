<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { issued } from '$lib/data';
  import { api } from '$lib/api';

  let status: 'checking' | 'paid' | 'failed' | 'error' = 'checking';
  let message = '';

  // SATIM redirects back here with ?orderId=... — confirm with the backend,
  // which issues the signed-QR tickets on success [M §7.4].
  onMount(async () => {
    const orderId = $page.url.searchParams.get('orderId');
    if (!orderId) { status = 'error'; message = 'orderId manquant'; return; }
    try {
      const res = await api.satimReturn(orderId);
      if (res.status === 'paid') { issued.set(res.tickets ?? []); status = 'paid'; setTimeout(() => goto('/ticket'), 900); }
      else { status = 'failed'; }
    } catch (e: any) { status = 'error'; message = e.message; }
  });
</script>

<div class="screen" style="text-align:center;padding-top:48px">
  {#if status === 'checking'}
    <h2 style="font-size:20px">Vérification du paiement…</h2>
    <p class="mut">Confirmation auprès de SATIM.</p>
  {:else if status === 'paid'}
    <div style="width:56px;height:56px;border-radius:50%;background:var(--ok);color:#fff;display:grid;place-items:center;margin:0 auto 12px;font-size:28px">✓</div>
    <h2 style="font-size:20px">Paiement accepté</h2>
    <p class="mut">Génération de vos billets…</p>
  {:else if status === 'failed'}
    <div style="width:56px;height:56px;border-radius:50%;background:var(--danger);color:#fff;display:grid;place-items:center;margin:0 auto 12px;font-size:28px">✕</div>
    <h2 style="font-size:20px">Paiement non abouti</h2>
    <button class="btn" style="margin-top:14px" on:click={() => goto('/')}>Retour à l'accueil</button>
  {:else}
    <h2 style="font-size:20px">Erreur</h2>
    <p class="mut">{message}</p>
    <button class="btn" style="margin-top:14px" on:click={() => goto('/')}>Retour</button>
  {/if}
</div>
