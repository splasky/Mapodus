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
  import { t } from '../i18n';

  let { onConnect, onPrev }: { onConnect: () => void; onPrev?: () => void } = $props();
  let umapUrl = $state('https://umap.openstreetmap.fr/en/');
  let username = $state('');
  let password = $state('');
  let passwordSaved = $state(false);
  let connecting = $state(false);
  let error = $state('');
  const MASKED_SECRET = '••••••••';

  $effect(() => {
    apiGet<{ umap_default_url: string; umap_account?: string; umap_password_saved: boolean }>('/settings')
      .then(status => {
        if (status.umap_default_url) {
          umapUrl = status.umap_default_url;
        }
        username = status.umap_account ?? '';
        passwordSaved = status.umap_password_saved;
        password = status.umap_password_saved ? MASKED_SECRET : '';
      })
      .catch(() => {
        apiGet<{ umap_url?: string }>('/umap/status')
          .then(status => {
            if (status.umap_url) {
              umapUrl = status.umap_url;
            }
          })
          .catch(() => {
            // Keep the built-in default if the status request fails.
          });
      });
  });

  async function connect() {
    if (!umapUrl || !username || (!password && !passwordSaved)) {
      error = passwordSaved
        ? t('connect.missingRequired')
        : t('connect.missingWithoutSavedPassword');
      return;
    }
    connecting = true;
    error = '';
    try {
      const passwordPayload = password === MASKED_SECRET ? '' : password;
      await apiPost('/umap/connect', { umap_url: umapUrl, username, password: passwordPayload });
      onConnect();
    } catch (e) {
      error = String(e);
    } finally {
      connecting = false;
    }
  }
</script>

<div class="card">
  <h2>{t('connect.title')}</h2>
  <p>{t('connect.description')}</p>

  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  <label>
    {t('connect.umapUrl')}
    <input bind:value={umapUrl} placeholder="https://umap.openstreetmap.fr/en/" />
  </label>
  <label>
    {t('connect.username')}
    <input bind:value={username} placeholder={t('connect.usernamePlaceholder')} />
  </label>
  <label>
    {t('connect.password')}
    <input
      type="password"
      bind:value={password}
      placeholder={passwordSaved ? t('connect.savedPasswordPlaceholder') : t('connect.passwordPlaceholder')}
    />
  </label>

  <div class="nav-row">
    <button class="nav-prev" onclick={onPrev}>{t('common.previous')}</button>
    <button class="primary" onclick={connect} disabled={connecting}>
      {connecting ? t('connect.connecting') : t('connect.connect')}
    </button>
  </div>
</div>
