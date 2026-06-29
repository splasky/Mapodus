<script lang="ts">
  import { apiPost } from '../api';

  let { onDone, onPrev }: { onDone: () => void; onPrev?: () => void } = $props();
  let transferring = $state(false);
  let error = $state('');
  let result = $state<{
    map_id?: string;
    map_url?: string;
    maps?: Array<{ name: string; map_id: string; map_url: string }>;
  } | null>(null);

  async function transfer() {
    transferring = true;
    error = '';
    try {
      // Fetch selected IDs from the bookmark list
      const bookmarks = await (await fetch('/api/bookmarks')).json();
      const selectedIds = Array.from(
        { length: bookmarks.bookmarks.length },
        (_, i) => i
      );

      const res = await apiPost<any>('/transfer', { selected_ids: selectedIds });
      result = res;
    } catch (e) {
      error = String(e);
    } finally {
      transferring = false;
    }
  }

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
              <a href={map.map_url} target="_blank" rel="noopener noreferrer">
                Open in uMap
              </a>
            </li>
          {/each}
        </ul>
      {:else}
        <p>Map created successfully!</p>
        <p>Map ID: {result.map_id}</p>
        <p>
          <a href={result.map_url} target="_blank" rel="noopener noreferrer">
            Open map in uMap
          </a>
        </p>
      {/if}
    </div>
    <button onclick={onDone}>Start Over</button>
  {:else}
    <p>Starting transfer...</p>
  {/if}

  <div class="nav-row">
    <button class="nav-prev" onclick={onPrev}>Previous</button>
    <span></span>
  </div>
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
