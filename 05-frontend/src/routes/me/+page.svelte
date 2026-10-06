<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { browser } from '$app/environment';
  import { api, isAuthed, clearTokens } from '$lib/api';

  let me: { email: string; full_name: string; locale: string; role: string } | null = null;
  let error = '', notice = '';
  // 2FA
  let setup: { otpauth_url: string; secret: string } | null = null;
  let code = '';
  // profile edit
  let fullName = '', locale = 'ar';
  // id doc + payment
  let idRef = '';
  let methods: Array<{ id: string; scheme: string; last4: string | null }> = [];
  let pmScheme = 'cib', pmToken = '', pmLast4 = '';

  onMount(load);
  async function load() {
    if (!isAuthed()) { if (browser) sessionStorage.setItem('after_login', '/me'); goto('/login'); return; }
    try {
      me = await api.me(); fullName = me.full_name; locale = me.locale;
      methods = (await api.listPaymentMethods()).methods;
    } catch (e: any) { error = e.message; }
  }

  async function saveProfile() { error=''; notice=''; try { await api.updateMe({ full_name: fullName, locale }); notice='Profil mis à jour.'; } catch(e:any){error=e.message;} }
  async function start2fa() { error=''; try { setup = await api.twofaSetup(); } catch(e:any){error=e.message;} }
  async function confirm2fa() { error=''; notice=''; try { const r=await api.twofaVerify(code); if(r.twofa_enabled){notice='2FA activée.'; setup=null;} } catch(e:any){error=e.message;} }
  async function saveId() { error=''; notice=''; try { await api.setIdDocument(idRef); notice='Pièce enregistrée.'; } catch(e:any){error=e.message;} }
  async function addPm() { error=''; notice=''; try { await api.addPaymentMethod({ scheme: pmScheme, gateway_token: pmToken, last4: pmLast4 || undefined }); pmToken=''; pmLast4=''; methods=(await api.listPaymentMethods()).methods; notice='Moyen de paiement ajouté.'; } catch(e:any){error=e.message;} }
  async function deleteAccount() {
    error=''; if (!browser) return;
    if (!window.confirm('Supprimer définitivement votre compte et vos données ?')) return;
    try { await api.deleteMe(); clearTokens(); goto('/'); } catch(e:any){error=e.message;}
  }
  function logout() { clearTokens(); goto('/'); }
</script>

<div class="screen">
  <h2 style="font-size:20px">Mon espace</h2>
  {#if error}<p style="color:var(--danger)">{error}</p>{/if}
  {#if notice}<p style="color:var(--ok)">{notice}</p>{/if}

  {#if me}
    <div class="summary" style="margin-top:12px">
      <div class="kv"><span class="k">Email</span><span>{me.email}</span></div>
      <div class="kv"><span class="k">Rôle</span><span>{me.role}</span></div>
    </div>

    <div class="section-title">Profil</div>
    <div class="summary" style="display:grid;gap:10px">
      <input bind:value={fullName} placeholder="Nom complet" style="padding:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
      <select bind:value={locale} style="padding:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)">
        <option value="ar">العربية</option><option value="fr">Français</option><option value="en">English</option>
      </select>
      <button class="btn" on:click={saveProfile}>Enregistrer</button>
    </div>

    <div class="section-title">Sécurité — 2FA</div>
    {#if !setup}
      <button class="btn" on:click={start2fa}>Activer la double authentification</button>
    {:else}
      <div class="summary" style="display:grid;gap:10px">
        <code style="word-break:break-all;font-size:11px;background:var(--surface-2);padding:8px;border-radius:8px">{setup.otpauth_url}</code>
        <div class="mut" style="font-size:12px">Secret : <strong>{setup.secret}</strong></div>
        <input placeholder="Code à 6 chiffres" bind:value={code} style="padding:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
        <button class="btn" on:click={confirm2fa}>Confirmer</button>
      </div>
    {/if}

    <div class="section-title">Pièce d'identité (facultatif)</div>
    <div class="summary" style="display:grid;gap:10px">
      <input bind:value={idRef} placeholder="Référence / numéro de pièce" style="padding:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
      <button class="btn" on:click={saveId}>Enregistrer</button>
    </div>

    <div class="section-title">Moyens de paiement</div>
    {#each methods as m}<div class="line"><span>{m.scheme.toUpperCase()}</span><span class="mut">•••• {m.last4 ?? '—'}</span></div>{/each}
    <div class="summary" style="display:grid;gap:10px">
      <select bind:value={pmScheme} style="padding:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)">
        <option value="cib">CIB</option><option value="dahabia">DAHABIA</option>
      </select>
      <input bind:value={pmToken} placeholder="Jeton passerelle (tokenisé)" style="padding:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
      <input bind:value={pmLast4} placeholder="4 derniers chiffres" maxlength="4" style="padding:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface);color:var(--fg)" />
      <button class="btn" on:click={addPm}>Ajouter</button>
      <p class="mut" style="font-size:11px;margin:0">Aucune donnée de carte complète n'est stockée (jeton uniquement).</p>
    </div>

    <div class="section-title">Billets</div>
    <a class="btn block" href="/me/tickets" style="text-align:center">Voir mes billets</a>

    <div style="margin-top:18px;display:flex;justify-content:space-between;align-items:center">
      <button class="back" style="margin:0" on:click={logout}>Se déconnecter</button>
      <button class="back" style="margin:0;color:var(--danger)" on:click={deleteAccount}>Supprimer mon compte</button>
    </div>
  {/if}
</div>
