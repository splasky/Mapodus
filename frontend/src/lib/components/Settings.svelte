<script lang="ts">
  import { apiGet, apiPost } from '../api';

  let { onBack }: { onBack: () => void } = $props();

  type SettingsResponse = {
    umap_default_url: string;
    umap_account?: string;
    locale: string;
    dev_mode: boolean;
    desktop_mode: boolean;
    umap_password_saved: boolean;
    google_maps_api_key_saved: boolean;
  };

  let loading = $state(true);
  let saving = $state(false);
  let error = $state('');
  let saved = $state('');
  let desktopMode = $state(false);
  let umapPasswordSaved = $state(false);
  let googleMapsApiKeySaved = $state(false);

  let umapDefaultUrl = $state('https://umap.openstreetmap.fr/en/');
  let umapAccount = $state('');
  let umapPassword = $state('');
  let googleMapsApiKey = $state('');
  let locale = $state('en');
  let devMode = $state(false);
  let clearUmapPassword = $state(false);
  let clearGoogleMapsApiKey = $state(false);

  async function load() {
    loading = true;
    error = '';
    try {
      const settings = await apiGet<SettingsResponse>('/settings');
      applySettings(settings);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function applySettings(settings: SettingsResponse) {
    umapDefaultUrl = settings.umap_default_url;
    umapAccount = settings.umap_account ?? '';
    locale = settings.locale;
    devMode = settings.dev_mode;
    desktopMode = settings.desktop_mode;
    umapPasswordSaved = settings.umap_password_saved;
    googleMapsApiKeySaved = settings.google_maps_api_key_saved;
    umapPassword = '';
    googleMapsApiKey = '';
    clearUmapPassword = false;
    clearGoogleMapsApiKey = false;
  }

  async function save() {
    saving = true;
    error = '';
    saved = '';
    try {
      const settings = await apiPost<SettingsResponse>('/settings', {
        umap_default_url: umapDefaultUrl,
        umap_account: umapAccount || null,
        locale,
        dev_mode: devMode,
        umap_password: umapPassword || null,
        clear_umap_password: clearUmapPassword,
        google_maps_api_key: googleMapsApiKey || null,
        clear_google_maps_api_key: clearGoogleMapsApiKey
      });
      applySettings(settings);
      saved = desktopMode
        ? 'Settings saved. Secrets are stored in the OS credential vault.'
        : 'Settings saved. Secrets are session-only in web/server mode.';
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  $effect(() => {
    load();
  });
</script>

<div class="card">
  <h2>Settings</h2>
  <p>Configure defaults used during migration. Passwords and API keys are never shown after saving.</p>

  {#if loading}
    <div class="notice">Loading settings...</div>
  {:else}
    {#if error}
      <div class="notice error">{error}</div>
    {/if}
    {#if saved}
      <div class="notice success">{saved}</div>
    {/if}

    <label>
      uMap URL
      <input bind:value={umapDefaultUrl} placeholder="https://umap.openstreetmap.fr/en/" />
    </label>

    <label>
      uMap account
      <input bind:value={umapAccount} placeholder="optional uMap username" />
    </label>

    <label>
      uMap password
      <input
        type="password"
        bind:value={umapPassword}
        placeholder={umapPasswordSaved ? 'Saved. Enter a new password to replace it.' : 'Optional'}
      />
    </label>
    {#if umapPasswordSaved}
      <label class="inline">
        <input type="checkbox" bind:checked={clearUmapPassword} />
        Remove saved uMap password
      </label>
    {/if}

    <label>
      Google Maps API key
      <input
        type="password"
        bind:value={googleMapsApiKey}
        placeholder={googleMapsApiKeySaved ? 'Saved. Enter a new key to replace it.' : 'Optional'}
      />
    </label>
    {#if googleMapsApiKeySaved}
      <label class="inline">
        <input type="checkbox" bind:checked={clearGoogleMapsApiKey} />
        Remove saved Google Maps API key
      </label>
    {/if}

    <label>
      Language
      <select bind:value={locale}>
        <option value="en">English</option>
        <option value="zh-TW">繁體中文</option>
      </select>
    </label>

    <label class="inline">
      <input type="checkbox" bind:checked={devMode} />
      Enable developer mode
    </label>

    <div class="settings-note">
      {#if desktopMode}
        Sensitive values are stored with your OS credential vault/keychain.
      {:else}
        Web/server mode keeps sensitive values in this browser session only.
      {/if}
    </div>

    <button class="primary" onclick={save} disabled={saving}>
      {saving ? 'Saving...' : 'Save settings'}
    </button>
  {/if}

  <div class="nav-row">
    <button class="nav-prev" onclick={onBack}>Back</button>
  </div>
</div>

<style>
  .inline {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 500;
  }

  .inline input {
    width: auto;
  }

  .settings-note {
    margin: 1rem 0;
    padding: 0.75rem;
    border-radius: 0.5rem;
    background: #f8fafc;
    color: #475569;
    font-size: 0.9rem;
  }
</style>
