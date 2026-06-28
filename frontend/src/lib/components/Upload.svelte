<script lang="ts">
  import { apiPost } from '../api';

  let { onUpload, onGoogleImport }: { onUpload: () => void; onGoogleImport?: () => void } = $props();
  let error = $state('');
  let uploading = $state(false);
  let dragging = $state(false);

  async function handleFile(file: File) {
    if (!file.name.endsWith('.csv')) {
      error = 'Please upload a CSV file (Google Takeout)';
      return;
    }
    uploading = true;
    error = '';
    try {
      const form = new FormData();
      form.append('file', file);
      await apiPost('/bookmarks/upload', form);
      onUpload();
    } catch (e) {
      error = String(e);
    } finally {
      uploading = false;
    }
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

  {#if onGoogleImport}
    <div class="divider"><span>or</span></div>
    <button class="alt-btn" onclick={onGoogleImport}>
      Import directly from Google Maps
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
  .alt-btn {
    width: 100%;
    padding: 0.7rem;
    border: 1px solid #cbd5e1;
    border-radius: 0.5rem;
    background: white;
    cursor: pointer;
    font-size: 0.9rem;
    transition: all 0.2s;
  }
  .alt-btn:hover {
    border-color: #2563eb;
    color: #2563eb;
  }
</style>
