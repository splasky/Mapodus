<script lang="ts">
  import { apiPost } from '../api';

  let { onUpload, onGoogleImport, onPrev, onNext }: { onUpload: () => void; onGoogleImport?: () => void; onPrev?: () => void; onNext?: () => void } = $props();
  let error = $state('');
  let uploading = $state(false);
  let dragging = $state(false);
  let uploaded = $state<any>(null);
  let enriching = $state(false);
  let enrichResult = $state<string | null>(null);
  let cookieInput = $state('');

  async function handleFile(file: File) {
    if (!file.name.endsWith('.csv')) {
      error = 'Please upload a CSV file (Google Takeout)';
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
      error = 'Paste your cookies first';
      return;
    }
    enriching = true;
    error = '';
    enrichResult = null;
    try {
      const parsed = parseCookies(cookieInput);
      const data = await apiPost<any>('/bookmarks/enrich', { cookies: parsed });
      enrichResult = `Enriched: ${data.enriched}, Skipped: ${data.skipped}`;
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
</script>

<div class="card">
  <h2>Upload Google Takeout CSV</h2>
  <p>Download your saved places from <a href="https://takeout.google.com" target="_blank">Google Takeout</a> and upload the CSV file here.</p>

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
      onclick={() => document.getElementById('file-input')?.click()}
      role="button"
      tabindex="0"
    >
      {#if uploading}
        Uploading...
      {:else}
        Drag & drop your CSV file here, or click to browse
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
      ✅ Uploaded {uploaded.bookmarks.length} places
    </div>

    {#if uploaded.validation}
      {#if uploaded.validation.ready === uploaded.validation.total}
        <div class="validation-ready">
          ✅ All {uploaded.validation.total} places have coordinates and are ready for uMap
        </div>
      {:else}
        <div class="validation-warning">
          ⚠️ {uploaded.validation.ready} of {uploaded.validation.total} places ready —
          {uploaded.validation.missing_coords.length} missing coordinates
          {#if uploaded.validation.missing_name.length > 0}
            , {uploaded.validation.missing_name.length} missing title
          {/if}
        </div>
      {/if}
    {/if}

    <details>
      <summary
        style="cursor: pointer; color: #2563eb; font-weight: 500; margin-top: 1rem;"
      >Optional: Enrich with Google Maps cookies</summary>
      <p class="hint">
        If your CSV is missing coordinates or addresses, paste your Google Maps cookies to attempt automatic enrichment.
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
        {enriching ? 'Enriching...' : 'Enrich with Google Maps'}
      </button>
      {#if enrichResult}
        <div class="notice success">{enrichResult}</div>
      {/if}
    </details>

    <div class="nav-row" style="margin-top: 1.5rem;">
      <button class="nav-prev" onclick={onPrev}>Previous</button>
      <button class="primary" onclick={onUpload}>Continue to Bookmarks</button>
      <button class="nav-next" onclick={onNext}>Next</button>
    </div>
  {/if}

  {#if onGoogleImport && !uploaded}
    <div class="divider"><span>or</span></div>
    <button class="google-btn" onclick={onGoogleImport}>
      Import directly from Google Maps
    </button>

    <div class="nav-row">
      <button class="nav-prev" onclick={onPrev}>Previous</button>
      <button class="nav-next" onclick={onNext}>Next</button>
    </div>
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
  .validation-ready {
    background: #ecfdf5;
    color: #065f46;
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    margin-top: 0.75rem;
    font-weight: 600;
    font-size: 0.9rem;
  }
  .validation-warning {
    background: #fef3c7;
    color: #92400e;
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    margin-top: 0.75rem;
    font-weight: 600;
    font-size: 0.9rem;
  }
</style>
