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
  import { apiPost } from '../api';
  import { t } from '../i18n';

  let { onImport }: { onImport: () => void } = $props();
  let cookieString = $state('');
  let error = $state('');
  let importing = $state(false);
  let lists = $state<Array<{name: string, count: number}>>([]);
  let selectedLists = $state<Set<string>>(new Set());
  let imported = $state(false);
  let transferMode = $state<'single' | 'per_list'>('single');

  function parseCookies(s: string): Record<string, string> {
    const cookies: Record<string, string> = {};
    // DevTools and copied cURL commands both provide semicolon-separated
    // key=value cookies. Preserve values after the first '=' because cookie
    // payloads can contain '=' characters.
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
      // Default to importing every list, then let the user opt out before the
      // server converts the selected lists into bookmark records.
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
        transfer_mode: transferMode,
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
    <h2>{t('googleImport.title')}</h2>
    <p>{t('googleImport.description')}</p>
    <p class="hint">{t('googleImport.cookieHint')}</p>
    {#if error}
      <div class="notice error">{error}</div>
    {/if}

    <label>
      <span class="label-text">{t('googleImport.cookieLabel')}</span>
      <textarea bind:value={cookieString} placeholder="SAPISID=...; SID=...; HSID=...; __Secure-1PSIDTS=...; ..." disabled={importing} rows={4}></textarea>
    </label>

    <button onclick={handleImport} disabled={importing || !cookieString.trim()}>
      {importing ? t('googleImport.fetching') : t('googleImport.fetch')}
    </button>
  {:else}
    <h2>{t('googleImport.selectTitle')}</h2>
    <p>{t('googleImport.selectDescription')}</p>

    {#if error}
      <div class="notice error">{error}</div>
    {/if}

    <div class="list-toolbar">
      <button onclick={() => selectedLists = new Set(lists.map(l => l.name))}>{t('googleImport.selectAll')}</button>
      <button onclick={() => selectedLists = new Set()}>{t('googleImport.selectNone')}</button>
      <span class="selected-count">
        {t('googleImport.selectedCount', { selected: selectedLists.size, total: lists.length })}
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
          <span class="list-count">{t('googleImport.places', { count: list.count })}</span>
        </div>
      {/each}
    </div>

    <div class="mode-select">
      <p class="label-text">{t('googleImport.transferMode')}</p>
      <div class="mode-options">
        <label>
          <input type="radio" bind:group={transferMode} value="single" disabled={importing} />
          <span>{t('googleImport.singleMap')}</span>
        </label>
        <label>
          <input type="radio" bind:group={transferMode} value="per_list" disabled={importing} />
          <span>{t('googleImport.perList')}</span>
        </label>
      </div>
    </div>

    <button onclick={handleConfirm} disabled={importing || selectedLists.size === 0}>
      {importing ? t('googleImport.saving') : t('googleImport.confirm')}
    </button>
    {#if selectedLists.size === 0}
      <p class="hint action-hint">{t('googleImport.selectRequiredHint')}</p>
    {/if}
  {/if}

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
  input {
    width: 100%;
    padding: 0.6rem;
    border: 1px solid #cbd5e1;
    border-radius: 0.5rem;
    font-family: monospace;
    font-size: 0.8rem;
    box-sizing: border-box;
  }
  .list-toolbar {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .selected-count {
    margin-left: auto;
    color: #64748b;
  }
  .list-group {
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
    margin-bottom: 1rem;
  }
  .list-row {
    display: grid;
    grid-template-columns: minmax(2rem, 2fr) minmax(0, 5fr) minmax(4rem, 3fr);
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 0.8rem;
    border-bottom: 1px solid #f1f5f9;
  }
  .list-row:last-child {
    border-bottom: none;
  }
  .list-row input,
  .mode-select input {
    width: auto;
  }
  .list-name {
    flex: 1;
    font-weight: 500;
  }
  .list-count {
    color: #64748b;
    font-size: 0.85rem;
    text-align: right;
  }
  .hint {
    font-size: 0.85rem;
    color: #64748b;
    margin-bottom: 0.8rem;
    line-height: 1.5;
  }
  .action-hint {
    text-align: center;
  }
  .mode-select {
    margin: 1rem 0;
    padding: 0.8rem;
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
    background: #f8fafc;
  }
  .mode-select .label-text {
    display: block;
    margin-bottom: 0.5rem;
  }
  .mode-options {
    display: grid;
    gap: 0.5rem;
  }
  .mode-select label {
    display: grid;
    grid-template-columns: minmax(2rem, 2fr) minmax(0, 3fr);
    align-items: center;
    justify-items: start;
    gap: 0.4rem;
    max-width: 24rem;
    font-weight: 400;
    cursor: pointer;
  }
</style>
