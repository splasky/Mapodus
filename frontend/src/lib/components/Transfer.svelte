<script lang="ts">
  import { apiGet, apiPost } from '../api';

  let { selectedIds, onDone }: { selectedIds: number[]; onDone: () => void } = $props();
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
      // Selection happens in the previous wizard step. Fail early here so the
      // backend only receives an explicit list of bookmark indexes to upload.
      if (!selectedIds.length) {
        throw new Error('No bookmarks selected for transfer');
      }
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
  <h2>Transfer to uMap</h2>

  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  {#if transferring}
    <p>Creating map and uploading bookmarks...</p>
    <progress></progress>
  {:else if result}
    <div class="notice success">
      {#if result.maps}
        <p>Created {result.maps.length} maps:</p>
        <ul class="map-list">
          {#each result.maps as map}
            <li>
              <strong>{map.name}</strong> —
              <a href={map.map_url} target="_blank" rel="noopener noreferrer" onclick={(event) => openMap(event, map.map_url)}>
                Open in uMap
              </a>
            </li>
          {/each}
        </ul>
      {:else}
        <p>Map created successfully!</p>
        <p>Map ID: {result.map_id}</p>
        <p>
          <a href={result.map_url} target="_blank" rel="noopener noreferrer" onclick={(event) => openMap(event, result?.map_url)}>
            Open map in uMap
          </a>
        </p>
      {/if}
    </div>
    <button onclick={onDone}>Upload another map 🗺️!</button>
  {:else}
    <p>Starting transfer...</p>
  {/if}

</div>

<style>
  .map-list {
    margin: 0.5rem 0;
    padding-left: 1.2rem;
  }
  .map-list li {
    margin: 0.4rem 0;
  }
</style>
