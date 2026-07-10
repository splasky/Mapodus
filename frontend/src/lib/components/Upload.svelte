<script lang="ts">
  import { apiPost } from '../api';
  import { t } from '../i18n';

  let { onUpload, onGoogleImport }: { onUpload: () => void; onGoogleImport?: () => void } = $props();
  let error = $state('');
  let uploading = $state(false);
  let dragging = $state(false);
  let uploaded = $state<any>(null);
  let enriching = $state(false);
  let enrichResult = $state<string | null>(null);
  let cookieInput = $state('');

  async function handleFile(file: File) {
    if (!file.name.endsWith('.csv')) {
      error = t('upload.csvOnly');
      return;
    }
    uploading = true;
    error = '';
    enrichResult = null;
    try {
      const form = new FormData();
      form.append('file', file);
      const data = await apiPost<any>('/bookmarks/upload', form);
      uploaded = data;
    } catch (e) {
      error = String(e);
    } finally {
      uploading = false;
    }
  }

  async function doEnrich() {
    if (!cookieInput.trim()) {
      error = t('upload.cookiesRequired');
      return;
    }
    enriching = true;
    error = '';
    enrichResult = null;
    try {
      const parsed = parseCookies(cookieInput);
      const data = await apiPost<any>('/bookmarks/enrich', { cookies: parsed });
      enrichResult = t('upload.enrichSummary', {
        enriched: data.enriched,
        skipped: data.skipped,
      });
      uploaded = { bookmarks: data.bookmarks, selected_ids: [] };
    } catch (e) {
      error = String(e);
    } finally {
      enriching = false;
    }
  }

  function parseCookies(raw: string): Record<string, string> {
    const result: Record<string, string> = {};
    for (const part of raw.split(';')) {
      const trimmed = part.trim();
      const eq = trimmed.indexOf('=');
      if (eq > 0) {
        result[trimmed.slice(0, eq)] = trimmed.slice(eq + 1);
      }
    }
    return result;
  }

  function openFilePicker() {
    document.getElementById('file-input')?.click();
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const file = e.dataTransfer?.files?.[0];
    if (file) handleFile(file);
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    dragging = true;
  }

  function onDragLeave() {
    dragging = false;
  }

  function onFileInput(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (file) handleFile(file);
  }

  function onDropZoneKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      openFilePicker();
    }
  }
</script>

<div class="card">
  <h2>{t('upload.title')}</h2>
  <p>
    {t('upload.descriptionBeforeLink')}<a href="https://takeout.google.com" target="_blank">{t('upload.takeoutLink')}</a>{t('upload.descriptionAfterLink')}
  </p>

  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  {#if !uploaded}
    <div
      class="drop-zone"
      class:dragging={dragging}
      ondragover={onDragOver}
      ondragleave={onDragLeave}
      ondrop={onDrop}
      onclick={openFilePicker}
      onkeydown={onDropZoneKeydown}
      role="button"
      tabindex="0"
    >
      {#if uploading}
        {t('upload.uploading')}
      {:else}
        {t('upload.dropHint')}
      {/if}
    </div>
    <input
      id="file-input"
      type="file"
      accept=".csv"
      onchange={onFileInput}
      style="display: none"
    />
  {:else}
    <div class="upload-success">
      {t('upload.uploaded', { count: uploaded.bookmarks.length })}
    </div>

    <details>
      <summary
        style="cursor: pointer; color: #2563eb; font-weight: 500; margin-top: 1rem;"
      >{t('upload.enrichTitle')}</summary>
      <p class="hint">
        {t('upload.enrichHint')}
      </p>
      <textarea
        bind:value={cookieInput}
        class="cookie-input"
        placeholder="SAPISID=...; SID=...; HSID=..."
        rows="3"
      ></textarea>
      <button
        class="enrich-btn"
        onclick={doEnrich}
        disabled={enriching}
      >
        {enriching ? t('upload.enriching') : t('upload.enrichAction')}
      </button>
      {#if enrichResult}
        <div class="notice success">{enrichResult}</div>
      {/if}
    </details>

    <div class="nav-row" style="margin-top: 1.5rem;">
      <button class="primary" onclick={onUpload}>{t('upload.continue')}</button>
    </div>
  {/if}

  {#if onGoogleImport && !uploaded}
    <div class="divider"><span>{t('upload.or')}</span></div>
    <button class="google-btn" onclick={onGoogleImport}>
      {t('upload.googleImport')}
    </button>

  {/if}
</div>

<style>
  .drop-zone {
    border: 2px dashed #cbd5e1;
    border-radius: 0.75rem;
    padding: 3rem 2rem;
    text-align: center;
    color: #64748b;
    cursor: pointer;
    transition: all 0.2s;
  }
  .drop-zone.dragging {
    border-color: #2563eb;
    background: #eff6ff;
    color: #2563eb;
  }
  .divider {
    display: flex;
    align-items: center;
    margin: 1.2rem 0;
    color: #94a3b8;
    font-size: 0.85rem;
  }
  .divider::before,
  .divider::after {
    content: '';
    flex: 1;
    border-bottom: 1px solid #e2e8f0;
  }
  .divider span {
    padding: 0 0.8rem;
  }
  .google-btn {
    width: 100%;
    padding: 0.75rem;
    border: none;
    border-radius: 0.5rem;
    background: #4285f4;
    color: white;
    cursor: pointer;
    font-size: 0.95rem;
    font-weight: 600;
    transition: background 0.2s;
  }
  .google-btn:hover {
    background: #3367d6;
  }
  .upload-success {
    background: #ecfdf5;
    color: #065f46;
    padding: 1rem;
    border-radius: 0.5rem;
    font-weight: 600;
    text-align: center;
  }
  .hint {
    font-size: 0.85rem;
    color: #64748b;
    margin: 0.5rem 0;
  }
  .cookie-input {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #cbd5e1;
    border-radius: 0.375rem;
    font-family: monospace;
    font-size: 0.8rem;
    box-sizing: border-box;
    resize: vertical;
  }
  .enrich-btn {
    margin-top: 0.5rem;
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 0.375rem;
    background: #6366f1;
    color: white;
    cursor: pointer;
    font-weight: 600;
    font-size: 0.85rem;
  }
  .enrich-btn:disabled {
    opacity: 0.6;
  }
</style>
