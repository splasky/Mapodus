<script lang="ts">
  import { apiGet } from '../api';

  let { onSelect, onPrev, onNext }: { onSelect: () => void; onPrev?: () => void; onNext?: () => void } = $props();
  let bookmarks = $state<any[]>([]);
  let selected = $state<Set<number>>(new Set());
  let loading = $state(true);
  let error = $state('');

  async function load() {
    try {
      const data = await apiGet<any>('/bookmarks');
      bookmarks = data.bookmarks;
      if (data.selected_ids?.length) {
        selected = new Set(data.selected_ids);
      } else {
        selected = new Set(bookmarks.map((_: any, i: number) => i));
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function toggle(i: number) {
    const next = new Set(selected);
    if (next.has(i)) next.delete(i);
    else next.add(i);
    selected = next;
  }

  function selectAll() {
    selected = new Set(bookmarks.map((_: any, i: number) => i));
  }

  function selectNone() {
    selected = new Set();
  }

  async function proceed() {
    // Store selection in session
    await fetch('/api/transfer', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ selected_ids: [...selected] }),
    }).catch(() => {});
    onSelect();
  }

  $effect(() => { load(); });
</script>

<div class="card">
  <h2>Select Bookmarks</h2>
  <p>Choose which places to transfer to uMap.</p>

  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  {#if loading}
    <p>Loading bookmarks...</p>
  {:else}
    <div style="display: flex; gap: 0.5rem; margin-bottom: 0.5rem;">
      <button onclick={selectAll}>Select All</button>
      <button onclick={selectNone}>Select None</button>
      <span style="margin-left: auto; color: #64748b;">
        {selected.size} / {bookmarks.length} selected
      </span>
    </div>

    <div class="bookmark-list">
      {#each bookmarks as bookmark, i}
        <div class="bookmark-row">
          <input type="checkbox" checked={selected.has(i)} onchange={() => toggle(i)} />
          <span class="bk-title">{bookmark.title || bookmark.place_name || 'Untitled'}</span>
          {#if bookmark.latitude && bookmark.longitude}
            <span class="bk-has-coords" title="Has coordinates">📍</span>
          {:else}
            <span class="bk-no-coords" title="Missing coordinates">⚠️</span>
          {/if}
        </div>
      {/each}
    </div>

    <button class="primary" onclick={proceed} disabled={selected.size === 0}>
      Transfer {selected.size} bookmarks to uMap
    </button>
  {/if}

  <div class="nav-row">
    <button class="nav-prev" onclick={onPrev}>Previous</button>
    <button class="nav-next" onclick={onNext}>Next</button>
  </div>
</div>
