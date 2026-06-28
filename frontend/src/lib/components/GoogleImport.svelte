<script lang="ts">
  import { apiPost } from '../api';

  let { onImport }: { onImport: () => void } = $props();
  let cookies = $state('');
  let error = $state('');
  let importing = $state(false);
  let lists = $state<Array<{name: string, count: number}>>([]);
  let selectedLists = $state<Set<string>>(new Set());
  let imported = $state(false);

  async function handleImport() {
    if (!cookies.trim()) {
      error = 'Please paste your Google Maps cookies';
      return;
    }

    importing = true;
    error = '';
    try {
      const parsed = parseCookies(cookies);
      const data = await apiPost<any>('/google/import', { cookies: parsed });
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

  function parseCookies(text: string): Record<string, string> {
    const result: Record<string, string> = {};
    for (const part of text.split(';')) {
      const eq = part.indexOf('=');
      if (eq > 0) {
        const key = part.substring(0, eq).trim();
        const val = part.substring(eq + 1).trim();
        if (key && val) result[key] = val;
      }
    }
    return result;
  }
</script>

<div class="card">
  {#if !imported}
    <h2>Import from Google Maps</h2>
    <p>Paste your Google Maps cookies below. You need <strong>SAPISID</strong> (or <strong>__Secure-1PSAPISID</strong>), <strong>SID</strong>, and <strong>HSID</strong> cookies.</p>
    <p class="hint">Open your browser's DevTools → Application → Cookies → <code>https://www.google.com</code>, copy all cookies as text.</p>

    {#if error}
      <div class="notice error">{error}</div>
    {/if}

    <textarea
      bind:value={cookies}
      placeholder="SAPISID=...; SID=...; HSID=..."
      rows={6}
      disabled={importing}
    ></textarea>

    <button onclick={handleImport} disabled={importing || !cookies.trim()}>
      {importing ? 'Importing...' : 'Import'}
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
</div>

<style>
  textarea {
    width: 100%;
    padding: 0.6rem;
    border: 1px solid #cbd5e1;
    border-radius: 0.5rem;
    font-family: monospace;
    font-size: 0.8rem;
    margin-bottom: 1rem;
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
  }
</style>
