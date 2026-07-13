<!--
  Copyright 2026 HYChang

  Licensed under the Apache License, Version 2.0 (the "License");
  you may not use this file except in compliance with the License.
  You may obtain a copy of the License at

      http://www.apache.org/licenses/LICENSE-2.0

  Unless required by applicable law or agreed to in writing, software
  distributed under the License is distributed on an "AS IS" BASIS,
  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
  See the License for the specific language governing permissions and
  limitations under the License.
-->

<script lang="ts">
  import { apiGet, apiPost } from '../api';
  import { setLocale, t } from '../i18n';

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
  const MASKED_SECRET = '••••••••';

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
    umapPassword = settings.umap_password_saved ? MASKED_SECRET : '';
    googleMapsApiKey = settings.google_maps_api_key_saved ? MASKED_SECRET : '';
    clearUmapPassword = false;
    clearGoogleMapsApiKey = false;
  }

  function secretPayload(value: string): string | null {
    return value === MASKED_SECRET ? null : value || null;
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
        umap_password: secretPayload(umapPassword),
        clear_umap_password: clearUmapPassword,
        google_maps_api_key: secretPayload(googleMapsApiKey),
        clear_google_maps_api_key: clearGoogleMapsApiKey
      });
      applySettings(settings);
      setLocale(settings.locale);
      saved = desktopMode ? t('settings.savedDesktop') : t('settings.savedWeb');
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
  <h2>{t('settings.title')}</h2>
  <p>{t('settings.description')}</p>

  {#if loading}
    <div class="notice">{t('settings.loading')}</div>
  {:else}
    {#if error}
      <div class="notice error">{error}</div>
    {/if}
    {#if saved}
      <div class="notice success">{saved}</div>
    {/if}

    <label>
      {t('settings.umapUrl')}
      <input bind:value={umapDefaultUrl} placeholder="https://umap.openstreetmap.fr/en/" />
    </label>

    <label>
      {t('settings.umapAccount')}
      <input bind:value={umapAccount} placeholder={t('settings.umapAccountPlaceholder')} />
    </label>

    <label>
      {t('settings.umapPassword')}
      <input
        type="password"
        bind:value={umapPassword}
        placeholder={umapPasswordSaved ? t('settings.secretSavedPlaceholder') : t('settings.optionalPlaceholder')}
      />
    </label>
    {#if umapPasswordSaved}
      <label class="inline">
        <input type="checkbox" bind:checked={clearUmapPassword} />
        {t('settings.removeUmapPassword')}
      </label>
    {/if}

    <label>
      {t('settings.googleMapsApiKey')}
      <input
        type="password"
        bind:value={googleMapsApiKey}
        placeholder={googleMapsApiKeySaved ? t('settings.secretSavedPlaceholder') : t('settings.optionalPlaceholder')}
      />
    </label>
    {#if googleMapsApiKeySaved}
      <label class="inline">
        <input type="checkbox" bind:checked={clearGoogleMapsApiKey} />
        {t('settings.removeGoogleMapsApiKey')}
      </label>
    {/if}

    <label>
      {t('settings.language')}
      <select bind:value={locale}>
        <option value="en">English</option>
        <option value="zh-TW">繁體中文</option>
      </select>
    </label>

    <label class="inline">
      <input type="checkbox" bind:checked={devMode} />
      {t('settings.devMode')}
    </label>

    <div class="settings-note">
      {#if desktopMode}
        {t('settings.desktopSecretNote')}
      {:else}
        {t('settings.webSecretNote')}
      {/if}
    </div>

    <button class="primary" onclick={save} disabled={saving}>
      {saving ? t('settings.saving') : t('settings.save')}
    </button>
  {/if}

  <div class="nav-row">
    <button class="nav-prev" onclick={onBack}>{t('common.back')}</button>
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
