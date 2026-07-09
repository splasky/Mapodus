<script lang="ts">
  import Upload from './lib/components/Upload.svelte';
  import GoogleImport from './lib/components/GoogleImport.svelte';
  import Bookmarks from './lib/components/Bookmarks.svelte';
  import ConnectUmap from './lib/components/ConnectUmap.svelte';
  import Settings from './lib/components/Settings.svelte';
  import Transfer from './lib/components/Transfer.svelte';

  // The app is intentionally a small wizard. Each step writes the server-side
  // session state needed by the next step instead of keeping all data in Svelte.
  let step = $state<'upload' | 'google-import' | 'bookmarks' | 'connect' | 'transfer' | 'settings'>('upload');
  let previousStep = $state<'upload' | 'google-import' | 'bookmarks' | 'connect' | 'transfer'>('upload');
  let umapConnected = $state(false);
  let bookmarksUploaded = $state(false);

  function onUpload() {
    bookmarksUploaded = true;
    step = 'bookmarks';
  }
  function onGoogleImport() {
    step = 'google-import';
  }
  function onImportDone() {
    bookmarksUploaded = true;
    step = 'bookmarks';
  }
  function onSelect() {
    step = 'connect';
  }
  function onConnect() {
    umapConnected = true;
    step = 'transfer';
  }
  function onDone() {
    step = 'upload';
    umapConnected = false;
    bookmarksUploaded = false;
  }
  function openSettings() {
    if (step !== 'settings') {
      previousStep = step;
    }
    step = 'settings';
  }
  function closeSettings() {
    step = previousStep;
  }

  function goPrev() {
    switch (step) {
      case 'google-import': step = 'upload'; break;
      case 'bookmarks': step = 'upload'; break;
      case 'connect': step = 'bookmarks'; break;
      case 'transfer': step = 'connect'; break;
    }
  }
  function goNext() {
    switch (step) {
      case 'upload': step = 'bookmarks'; break;
      case 'google-import': step = 'bookmarks'; break;
      case 'bookmarks': step = 'connect'; break;
      case 'connect': step = 'transfer'; break;
    }
  }
</script>

<div class="layout">
  <div class="hero">
    <button class="settings-button" onclick={openSettings} aria-label="Open settings">⚙️</button>
    <h1>google-maps-to-umap</h1>
    <p>Convert Google Maps saved places to uMap</p>
  </div>

  {#if step !== 'settings'}
    <div class="steps">
    <div class="step {step === 'upload' || step === 'google-import' ? 'active' : ''} {bookmarksUploaded ? 'done' : ''}">
      <span class="step-num">1</span> Import
    </div>
    <div class="step {step === 'bookmarks' ? 'active' : ''} {bookmarksUploaded ? 'done' : ''}">
      <span class="step-num">2</span> Select Bookmarks
    </div>
    <div class="step {step === 'connect' ? 'active' : ''} {umapConnected ? 'done' : ''}">
      <span class="step-num">3</span> Connect uMap
    </div>
    <div class="step {step === 'transfer' ? 'active' : ''}">
      <span class="step-num">4</span> Transfer
    </div>
  </div>
  {/if}

  {#if step === 'settings'}
    <Settings onBack={closeSettings} />
  {:else if step === 'upload'}
    <Upload {onUpload} {onGoogleImport} onPrev={goPrev} onNext={goNext} />
  {:else if step === 'google-import'}
    <GoogleImport onImport={onImportDone} onPrev={goPrev} onNext={goNext} />
  {:else if step === 'bookmarks'}
    <Bookmarks onSelect={onSelect} onPrev={goPrev} onNext={goNext} />
  {:else if step === 'connect'}
    <ConnectUmap onConnect={onConnect} onPrev={goPrev} onNext={goNext} />
  {:else if step === 'transfer'}
    <Transfer onDone={onDone} onPrev={goPrev} />
  {/if}
</div>

<style>
  .steps {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 2rem;
    flex-wrap: wrap;
  }

  .hero {
    position: relative;
  }

  .settings-button {
    position: absolute;
    top: 0;
    right: 0;
    width: auto;
    border: 1px solid #cbd5e1;
    border-radius: 999px;
    background: white;
    color: #334155;
    cursor: pointer;
    font-size: 1.1rem;
    padding: 0.4rem 0.6rem;
  }

  .settings-button:hover {
    border-color: #2563eb;
    color: #2563eb;
  }
  .step {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.8rem;
    border-radius: 2rem;
    background: #e2e8f0;
    color: #64748b;
    font-size: 0.85rem;
  }
  .step.active {
    background: #2563eb;
    color: white;
  }
  .step.done {
    background: #dcfce7;
    color: #166534;
  }
  .step-num {
    display: inline-flex;
    width: 1.3rem;
    height: 1.3rem;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: rgba(0,0,0,0.1);
    font-size: 0.75rem;
    font-weight: bold;
  }

  :global(.nav-row) {
    display: flex;
    justify-content: space-between;
    margin-top: 1.5rem;
    gap: 0.5rem;
  }
  :global(.nav-prev), :global(.nav-next) {
    padding: 0.5rem 1rem;
    border: 1px solid #cbd5e1;
    border-radius: 0.4rem;
    background: white;
    cursor: pointer;
    font-size: 0.85rem;
    color: #475569;
    transition: all 0.15s;
  }
  :global(.nav-prev:hover), :global(.nav-next:hover) {
    border-color: #2563eb;
    color: #2563eb;
  }
  :global(button.primary) {
    width: 100%;
    padding: 0.7rem;
    border: none;
    border-radius: 0.5rem;
    background: #2563eb;
    color: white;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
    transition: background 0.2s;
  }
  :global(button.primary:hover) {
    background: #1d4ed8;
  }
  :global(button.primary:disabled) {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
