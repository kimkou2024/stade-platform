<script lang="ts">
  import '../app.css';
  import { locale, t, RTL, type Locale } from '$lib/i18n';
  import { browser } from '$app/environment';

  let theme: 'dark' | 'light' | null = null;

  $: if (browser) {
    document.documentElement.lang = $locale;
    document.documentElement.dir = RTL[$locale] ? 'rtl' : 'ltr';
  }
  function setLocale(l: Locale) { locale.set(l); }
  function toggleTheme() {
    theme = theme === 'dark' ? 'light' : theme === 'light' ? null : 'dark';
    if (!browser) return;
    if (theme) document.documentElement.setAttribute('data-theme', theme);
    else document.documentElement.removeAttribute('data-theme');
  }
</script>

<div class="app">
  <div class="bar">
    <a class="brand" href="/"><span class="crest">SA</span>{$t('app')}</a>
    <span class="spacer"></span>
    <a href="/admin" class="lang" style="text-decoration:none">Admin</a>
    <span style="display:flex;gap:4px">
      <button class="lang" aria-pressed={$locale === 'ar'} on:click={() => setLocale('ar')}>ع</button>
      <button class="lang" aria-pressed={$locale === 'fr'} on:click={() => setLocale('fr')}>FR</button>
      <button class="lang" aria-pressed={$locale === 'en'} on:click={() => setLocale('en')}>EN</button>
    </span>
    <button class="icon-btn" title="Thème" on:click={toggleTheme}>◐</button>
  </div>
  <slot />
</div>
