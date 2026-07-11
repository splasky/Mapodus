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

  $effect(() => {
    apiGet<{ umap_default_url: string; umap_account?: string; umap_password_saved: boolean }>('/settings')
      .then(status => {
        if (status.umap_default_url) {
          umapUrl = status.umap_default_url;
        }
        username = status.umap_account ?? '';
        passwordSaved = status.umap_password_saved;
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
      await apiPost('/umap/connect', { umap_url: umapUrl, username, password });
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

    <button class="primary" onclick={connect} disabled={connecting}>
    {connecting ? t('connect.connecting') : t('connect.connect')}
  </button>

  <div class="nav-row single-action">
    <button class="nav-prev" onclick={onPrev}>{t('common.previous')}</button>
  </div>
</div>

<style>
  .single-action {
    justify-content: flex-start;
  }
</style>
