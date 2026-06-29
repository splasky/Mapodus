<script lang="ts">
  import { apiPost } from '../api';

  let { onImport, onPrev, onNext }: { onImport: () => void; onPrev?: () => void; onNext?: () => void } = $props();
  let cookieString = $state('');
  let error = $state('');
  let importing = $state(false);
  let lists = $state<Array<{name: string, count: number}>>([]);
  let selectedLists = $state<Set<string>>(new Set());
  let imported = $state(false);

  function parseCookies(s: string): Record<string, string> {
    const cookies: Record<string, string> = {};
    for (const pair of s.split(';')) {
      const eq = pair.indexOf('=');
      if (eq === -1) continue;
      const key = pair.slice(0, eq).trim();
      const val = pair.slice(eq + 1).trim();
      if (key && val) cookies[key] = val;
    }
    return cookies;
  }

  async function handleImport() {
    importing = true;
    error = '';
    try {
      const cookies = parseCookies(cookieString);
      const data = await apiPost<any>('/google/import', { cookies });
      lists = data.lists;
      selectedLists = new Set(data.lists.map((l: any) => l.name));
      imported = true;
    } catch (e) {
      error = String(e);
    } finally {
      importing = false;
    }
  }

  function toggleList(name: string) {
    const next = new Set(selectedLists);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    selectedLists = next;
  }

  async function handleConfirm() {
    importing = true;
    error = '';
    try {
      await apiPost('/google/confirm', {
        selected_lists: [...selectedLists],
      });
      onImport();
    } catch (e) {
      error = String(e);
    } finally {
      importing = false;
    }
  }
</script>

<div class="card">
  {#if !imported}
    <h2>Import from Google Maps</h2>
    <p>Paste your Google cookies below. Cookies expire after a few hours — collect fresh ones before each import.</p>
    <p class="hint">Open DevTools (<kbd>F12</kbd>) → Application → Cookies → <code>https://www.google.com</code>. Right-click any cookie → <strong>Copy All</strong>, or copy the <code>-b</code> argument from a cURL command. Paste the raw cookie string here.</p>

    {#if error}
      <div class="notice error">{error}</div>
    {/if}

    <label>
      <span class="label-text">Cookie string (semicolon-separated <code>key=value</code> pairs)</span>
      <textarea bind:value={cookieString} placeholder="SAPISID=...; SID=...; HSID=...; __Secure-1PSIDTS=...; ..." disabled={importing} rows={4}></textarea>
    </label>

    <button onclick={handleImport} disabled={importing || !cookieString.trim()}>
      {importing ? 'Fetching lists...' : 'Fetch My Saved Lists'}
    </button>
  {:else}
    <h2>Select Lists to Import</h2>
    <p>Choose which saved lists to import from Google Maps.</p>

    {#if error}
      <div class="notice error">{error}</div>
    {/if}

    <div style="display: flex; gap: 0.5rem; margin-bottom: 0.5rem;">
      <button onclick={() => selectedLists = new Set(lists.map(l => l.name))}>Select All</button>
      <button onclick={() => selectedLists = new Set()}>Select None</button>
      <span style="margin-left: auto; color: #64748b;">
        {selectedLists.size} / {lists.length} lists selected
      </span>
    </div>

    <div class="list-group">
      {#each lists as list}
        <div class="list-row">
          <input
            type="checkbox"
            checked={selectedLists.has(list.name)}
            onchange={() => toggleList(list.name)}
          />
          <span class="list-name">{list.name}</span>
          <span class="list-count">{list.count} places</span>
        </div>
      {/each}
    </div>

    <button onclick={handleConfirm} disabled={importing || selectedLists.size === 0}>
      {importing ? 'Saving...' : 'Import Selected to uMap'}
    </button>
  {/if}

  <div class="nav-row">
    <button class="nav-prev" onclick={onPrev}>Previous</button>
    <button class="nav-next" onclick={onNext}>Next</button>
  </div>
</div>

<style>
  label {
    display: grid;
    gap: 0.3rem;
    margin-bottom: 0.8rem;
  }
  .label-text {
    font-size: 0.85rem;
    font-weight: 600;
    color: #334155;
  }
  .label-text code {
    font-weight: 400;
    color: #64748b;
  }
  input {
    width: 100%;
    padding: 0.6rem;
    border: 1px solid #cbd5e1;
    border-radius: 0.5rem;
    font-family: monospace;
    font-size: 0.8rem;
    box-sizing: border-box;
  }
  .list-group {
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
    margin-bottom: 1rem;
  }
  .list-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 0.8rem;
    border-bottom: 1px solid #f1f5f9;
  }
  .list-row:last-child {
    border-bottom: none;
  }
  .list-name {
    flex: 1;
    font-weight: 500;
  }
  .list-count {
    color: #64748b;
    font-size: 0.85rem;
  }
  .hint {
    font-size: 0.85rem;
    color: #64748b;
    margin-bottom: 0.8rem;
    line-height: 1.5;
  }
</style>
