<script lang="ts">
  import { apiGet, apiPost } from '../api';
  import { t } from '../i18n';

  let { onUpload, onGoogleImport }: { onUpload: () => void; onGoogleImport?: () => void } = $props();
  let error = $state('');
  let uploading = $state(false);
  let dragging = $state(false);
  let uploaded = $state<any>(null);
  let desktopMode = $state(false);

  async function loadDesktopMode() {
    try {
      const settings = await apiGet<{ desktop_mode: boolean }>('/settings');
      desktopMode = settings.desktop_mode;
    } catch {
      desktopMode = false;
    }
  }

  async function openTakeout(event: MouseEvent) {
    if (!desktopMode) return;
    event.preventDefault();
    try {
      await apiPost('/open-external', { url: 'https://takeout.google.com/' });
    } catch (e) {
      error = String(e);
    }
  }

  async function handleFile(file: File) {
    if (!file.name.endsWith('.csv')) {
      error = t('upload.csvOnly');
      return;
    }
    uploading = true;
    error = '';
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

  $effect(() => { loadDesktopMode(); });

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
    {t('upload.descriptionBeforeLink')}<a href="https://takeout.google.com/" target="_blank" rel="noopener noreferrer" onclick={openTakeout}>{t('upload.takeoutLink')}</a>{t('upload.descriptionAfterLink')}
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

    {#if uploaded.validation}
      {#if uploaded.validation.ready === uploaded.validation.total}
        <div class="validation-ready">
          ✅ {t('upload.validationReady', { total: uploaded.validation.total })}
        </div>
      {:else}
        <div class="validation-warning">
          ⚠️ {t('upload.validationWarning', { ready: uploaded.validation.ready, total: uploaded.validation.total, missingCoords: uploaded.validation.missing_coords.length })}
          {#if uploaded.validation.missing_name.length > 0}
            {t('upload.validationMissingName', { count: uploaded.validation.missing_name.length })}
          {/if}
        </div>
      {/if}
    {/if}

    <div class="nav-row" style="margin-top: 1.5rem;">
      <button class="primary" onclick={onUpload}>{t('upload.continue')}</button>
    </div>
  {/if}

  {#if onGoogleImport && !uploaded}
    <div class="divider"><span>{t('upload.or')}</span></div>
    <button class="google-btn" onclick={onGoogleImport}>
      {t('upload.googleImport')}
    </button>
    <p class="hint source-hint">{t('upload.sourceHint')}</p>
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
  .source-hint {
    text-align: center;
  }
</style>
