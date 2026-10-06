<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';

  let step: 'request' | 'set' = 'request';
  let email = '', token = '', newPass = '', error = '', notice = '', devToken = '';

  onMount(() => {
    const t = $page.url.searchParams.get('token');
    if (t) { token = t; step = 'set'; }
  });

  async function request() {
    error = ''; notice = '';
    try {
      const r = await api.passwordForgot(email);
      notice = 'Si un compte existe, un lien de réinitialisation a été envoyé.';
      if (r.dev_token) { devToken = r.dev_token; token = r.dev_token; } // dev: no email provider yet
    } catch (e: any) { error = e.message; }
  }
  async function reset() {
    error = ''; notice = '';
    try { await api.passwordReset(token, newPass); notice = 'Mot de passe modifié.'; setTimeout(() => goto('/login'), 900); }
    catch (e: any) { error = e.message; }
  }
</script>

<div class="screen">
  <h2 style="font-size:20px">Réinitialiser le mot de passe</h2>
  {#if error}<p style="color:var(--danger)">{error}</p>{/if}
  {#if notice}<p style="color:var(--ok)">{notice}</p>{/if}

  {#if step === 'request'}
    <div style="display:grid;gap:10px;max-width:340px;margin-top:12px">
      <input placeholder="email" type="email" bind:value={email} style="padding:11px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
      <button class="btn" on:click={request}>Envoyer le lien</button>
      {#if devToken}
        <p class="mut" style="font-size:12px">Dev (pas d'e-mail configuré) : <a href={`/reset?token=${devToken}`}>continuer la réinitialisation</a></p>
      {/if}
    </div>
  {:else}
    <div style="display:grid;gap:10px;max-width:340px;margin-top:12px">
      <input placeholder="Nouveau mot de passe" type="password" bind:value={newPass} style="padding:11px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
      <button class="btn" on:click={reset}>Changer le mot de passe</button>
    </div>
  {/if}
</div>
