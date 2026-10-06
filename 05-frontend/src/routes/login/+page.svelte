<script lang="ts">
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';
  import { t } from '$lib/i18n';
  import { api, setTokens } from '$lib/api';

  let mode: 'login' | 'register' = 'login';
  let email = '', password = '', full_name = '', totp = '', error = '', busy = false;
  let consent = false;

  async function submit() {
    error = ''; busy = true;
    try {
      if (mode === 'register') {
        if (!consent) { error = 'Vous devez accepter le traitement des données (Loi 18-07).'; busy = false; return; }
        await api.register({ email, password, full_name, consent });
      }
      const r = await api.login({ email, password, totp_code: totp || undefined });
      setTokens(r.access_token, r.refresh_token);
      const back = browser ? sessionStorage.getItem('after_login') : null;
      if (browser && back) sessionStorage.removeItem('after_login');
      goto(back || '/');
    } catch (e: any) { error = e.message; } finally { busy = false; }
  }
</script>

<div class="screen">
  <h2 style="font-size:20px">{mode === 'login' ? $t('login') : 'Créer un compte'}</h2>
  <div style="display:grid;gap:10px;max-width:340px;margin-top:12px">
    {#if mode === 'register'}
      <input placeholder="Nom complet" bind:value={full_name} style="padding:11px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
    {/if}
    <input placeholder="email" type="email" bind:value={email} style="padding:11px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
    <input placeholder="mot de passe" type="password" bind:value={password} style="padding:11px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
    <input placeholder="Code 2FA (si activé)" bind:value={totp} style="padding:11px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
    {#if mode === 'register'}
      <label style="font-size:12px;display:flex;gap:8px;align-items:flex-start">
        <input type="checkbox" bind:checked={consent} />
        <span>J'accepte le traitement de mes données conformément à la <a href="/privacy">Loi 18-07</a>.</span>
      </label>
    {/if}
    <button class="btn block" disabled={busy} on:click={submit}>{busy ? '…' : (mode === 'login' ? $t('login') : 'Créer')}</button>
    <button class="back" on:click={() => (mode = mode === 'login' ? 'register' : 'login')} style="margin:0">
      {mode === 'login' ? "Pas de compte ? S'inscrire" : 'Déjà un compte ? Se connecter'}
    </button>
    {#if mode === 'login'}<a class="back" href="/reset" style="margin:0">Mot de passe oublié ?</a>{/if}
    {#if error}<p style="color:var(--danger)">{error}</p>{/if}
  </div>
</div>
