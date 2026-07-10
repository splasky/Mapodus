<script lang="ts">
  import { apiGet, apiPost } from '../api';
  import { t } from '../i18n';

  let { onSelect, onPrev, onNext }: { onSelect: () => void; onPrev?: () => void; onNext?: () => void } = $props();
  let bookmarks = $state<any[]>([]);
  let selected = $state<Set<number>>(new Set());
  let loading = $state(true);
  let transferring = $state(false);
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
    transferring = true;
    error = '';
    try {
      await apiPost('/bookmarks/auto_enrich').catch(() => {});
      await apiPost('/bookmarks/select', { selected_ids: [...selected] }).catch(() => {});
      onSelect();
    } catch (e) {
      error = String(e);
    } finally {
      transferring = false;
    }
  }

  $effect(() => { load(); });
</script>

<div class="card">
  <h2>{t('bookmarks.title')}</h2>
  <p>{t('bookmarks.description')}</p>

  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  {#if loading}
    <p>{t('bookmarks.loading')}</p>
  {:else}
    <div class="bookmark-toolbar">
      <button onclick={selectAll}>{t('bookmarks.selectAll')}</button>
      <button onclick={selectNone}>{t('bookmarks.selectNone')}</button>
      <span class="selected-count">
        {t('bookmarks.selectedCount', { selected: selected.size, total: bookmarks.length })}
      </span>
    </div>

    <div class="bookmark-list">
      {#each bookmarks as bookmark, i}
        <div class="bookmark-row">
          <input type="checkbox" checked={selected.has(i)} onchange={() => toggle(i)} />
          <span class="bk-title">{bookmark.title || bookmark.place_name || t('bookmarks.untitled')}</span>
          {#if bookmark.latitude && bookmark.longitude}
            <span class="bk-has-coords" title={t('bookmarks.hasCoordinates')}>📍</span>
          {:else}
            <span class="bk-no-coords" title={t('bookmarks.missingCoordinates')}>⚠️</span>
          {/if}
        </div>
      {/each}
    </div>

    <button class="primary" onclick={proceed} disabled={selected.size === 0 || transferring}>
      {#if transferring}
        <span class="spinner"></span> {t('bookmarks.transferring')}
      {:else}
        {t('bookmarks.transferAction', { count: selected.size })}
      {/if}
    </button>
  {/if}

  <div class="nav-row">
    <button class="nav-prev" onclick={onPrev}>{t('common.previous')}</button>
    <button class="nav-next" onclick={onNext}>{t('common.next')}</button>
  </div>
</div>

<style>
  .bookmark-toolbar {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .selected-count {
    margin-left: auto;
    color: #64748b;
  }
  .spinner {
    display: inline-block;
    width: 1rem;
    height: 1rem;
    border: 2px solid rgba(255,255,255,0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
    vertical-align: middle;
    margin-right: 0.4rem;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  .bookmark-list {
    max-height: 400px;
    overflow-y: auto;
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
  }
  .bookmark-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #f1f5f9;
  }
  .bookmark-row:last-child {
    border-bottom: none;
  }
  .bk-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bk-has-coords,
  .bk-no-coords {
    font-size: 0.85rem;
  }
</style>
