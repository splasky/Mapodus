<script lang="ts">
  import Login from './lib/components/Login.svelte';
  import Upload from './lib/components/Upload.svelte';
  import GoogleImport from './lib/components/GoogleImport.svelte';
  import Bookmarks from './lib/components/Bookmarks.svelte';
  import ConnectUmap from './lib/components/ConnectUmap.svelte';
  import Transfer from './lib/components/Transfer.svelte';

  let step = $state<'login' | 'upload' | 'google-import' | 'bookmarks' | 'connect' | 'transfer'>('login');
  let loggedIn = $state(false);
  let umapConnected = $state(false);
  let bookmarksUploaded = $state(false);

  function onLogin() {
    loggedIn = true;
    step = 'upload';
  }
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
    step = 'login';
    loggedIn = false;
    umapConnected = false;
    bookmarksUploaded = false;
  }

  function goPrev() {
    switch (step) {
      case 'upload': case 'google-import': step = 'login'; break;
      case 'bookmarks': step = 'upload'; break;
      case 'connect': step = 'bookmarks'; break;
      case 'transfer': step = 'connect'; break;
    }
  }
  function goNext() {
    switch (step) {
      case 'login': step = 'upload'; break;
      case 'upload': step = 'bookmarks'; break;
      case 'google-import': step = 'bookmarks'; break;
      case 'bookmarks': step = 'connect'; break;
      case 'connect': step = 'transfer'; break;
    }
  }
</script>

<div class="layout">
  <div class="hero">
    <h1>google-maps-to-umap</h1>
    <p>Convert Google Maps saved places to uMap</p>
  </div>

  <div class="steps">
    <div class="step {step === 'login' ? 'active' : ''} {loggedIn ? 'done' : ''}">
      <span class="step-num">1</span> Sign In
    </div>
    <div class="step {step === 'upload' || step === 'google-import' ? 'active' : ''} {bookmarksUploaded ? 'done' : ''}">
      <span class="step-num">2</span> Import
    </div>
    <div class="step {step === 'bookmarks' ? 'active' : ''} {bookmarksUploaded ? 'done' : ''}">
      <span class="step-num">3</span> Select Bookmarks
    </div>
    <div class="step {step === 'connect' ? 'active' : ''} {umapConnected ? 'done' : ''}">
      <span class="step-num">4</span> Connect uMap
    </div>
    <div class="step {step === 'transfer' ? 'active' : ''}">
      <span class="step-num">5</span> Transfer
    </div>
  </div>

  {#if step === 'login'}
    <Login {onLogin} onNext={goNext} />
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
