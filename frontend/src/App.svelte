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
    <div class="step {step === 'bookmarks' ? 'active' : ''}">
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
    <Login onLogin={onLogin} />
  {:else if step === 'upload'}
    <Upload {onUpload} {onGoogleImport} />
  {:else if step === 'google-import'}
    <GoogleImport onImport={onImportDone} />
  {:else if step === 'bookmarks'}
    <Bookmarks onSelect={onSelect} />
  {:else if step === 'connect'}
    <ConnectUmap onConnect={onConnect} />
  {:else if step === 'transfer'}
    <Transfer onDone={onDone} />
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
</style>
