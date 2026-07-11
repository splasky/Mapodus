<script lang="ts">
  import { apiGet, apiPost } from '../api';
  import { t } from '../i18n';

  let { onDone, onPrev }: { onDone: () => void; onPrev?: () => void } = $props();
  let transferring = $state(false);
  let error = $state('');
  let desktopMode = $state(false);
  let result = $state<{
    map_id?: string;
    map_url?: string;
    maps?: Array<{ name: string; map_id: string; map_url: string }>;
  } | null>(null);

  async function loadDesktopMode() {
    try {
      const settings = await apiGet<{ desktop_mode: boolean }>('/settings');
      desktopMode = settings.desktop_mode;
    } catch {
      desktopMode = false;
    }
  }

  async function transfer() {
    transferring = true;
    error = '';
    try {
      // Selection is stored by the previous wizard step so transfer can reload it here.
      const bookmarks = await apiGet<{ bookmarks: any[]; selected_ids: number[] }>('/bookmarks');
      const selectedIds = bookmarks.selected_ids.length > 0
        ? bookmarks.selected_ids
        : bookmarks.bookmarks.map((_: any, i: number) => i);

      const res = await apiPost<any>('/transfer', { selected_ids: selectedIds });
      result = res;
    } catch (e) {
      error = String(e);
    } finally {
      transferring = false;
    }
  }

  async function openMap(event: MouseEvent, url: string | undefined) {
    if (!desktopMode || !url) return;
    event.preventDefault();
    try {
      // Desktop mode uses the embedded backend to hand off uMap URLs to the OS
      // browser. Web mode keeps the normal anchor behavior.
      await apiPost('/open-external', { url });
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => { loadDesktopMode(); });
  $effect(() => { transfer(); });
</script>

<div class="card">
  <h2>{t('transfer.title')}</h2>

  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  {#if transferring}
    <div class="transfer-progress" role="status" aria-live="polite">
      <span class="spinner" aria-hidden="true"></span>
      <p>{t('transfer.progress')}</p>
    </div>
    <progress></progress>
  {:else if result}
    <div class="notice success">
      {#if result.maps}
        <p>{t('transfer.createdMaps', { count: result.maps.length })}</p>
        <ul class="map-list">
          {#each result.maps as map}
            <li>
              <strong>{map.name}</strong> —
              <a href={map.map_url} target="_blank" rel="noopener noreferrer" onclick={(event) => openMap(event, map.map_url)}>
                {t('transfer.openInUmap')}
              </a>
            </li>
          {/each}
        </ul>
      {:else}
        <p>{t('transfer.success')}</p>
        <p>{t('transfer.mapId', { id: result.map_id ?? '' })}</p>
        <p>
          <a href={result.map_url} target="_blank" rel="noopener noreferrer" onclick={(event) => openMap(event, result?.map_url)}>
            {t('transfer.openInUmap')}
          </a>
        </p>
      {/if}
    </div>
    <button onclick={onDone}>{t('transfer.uploadAnother')}</button>
  {:else}
    <p>{t('transfer.starting')}</p>
  {/if}

</div>

<style>
  .transfer-progress {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin: 0.75rem 0;
  }
  .transfer-progress p {
    margin: 0;
  }
  .spinner {
    width: 1.2rem;
    height: 1.2rem;
    flex: 0 0 auto;
    border: 3px solid rgba(36, 79, 60, 0.18);
    border-top-color: #376d52;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  .map-list {
    margin: 0.5rem 0;
    padding-left: 1.2rem;
  }
  .map-list li {
    margin: 0.4rem 0;
  }
</style>
