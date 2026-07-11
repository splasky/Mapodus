<script lang="ts">
  import Upload from './lib/components/Upload.svelte';
  import GoogleImport from './lib/components/GoogleImport.svelte';
  import Bookmarks from './lib/components/Bookmarks.svelte';
  import ConnectUmap from './lib/components/ConnectUmap.svelte';
  import Settings from './lib/components/Settings.svelte';
  import Transfer from './lib/components/Transfer.svelte';
  import About from './lib/components/About.svelte';
  import { apiGet } from './lib/api';
  import { setLocale, t } from './lib/i18n';

  // The app is intentionally a small wizard. Each step writes the server-side
  // session state needed by the next step instead of keeping all data in Svelte.
  let step = $state<'upload' | 'google-import' | 'bookmarks' | 'connect' | 'transfer' | 'settings'>('upload');
  let previousStep = $state<'upload' | 'google-import' | 'bookmarks' | 'connect' | 'transfer'>('upload');
  let aboutOpen = $state(false);
  let umapConnected = $state(false);
  let bookmarksUploaded = $state(false);

  $effect(() => {
    apiGet<{ locale?: string }>('/settings')
      .then(settings => setLocale(settings.locale))
      .catch(() => setLocale(null));
  });

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

  function openAbout() {
    aboutOpen = true;
  }

  function goPrev() {
    switch (step) {
      case 'google-import': step = 'upload'; break;
      case 'bookmarks': step = 'upload'; break;
      case 'connect': step = 'bookmarks'; break;
      case 'transfer': step = 'connect'; break;
    }
  }
</script>

<div class="layout">
  <div class="hero">
    <div class="hero-actions" aria-label="Mapodus actions">
      <button class="hero-action" onclick={openAbout} aria-label={t('about.open')}>ℹ️</button>
      <button class="hero-action" onclick={openSettings} aria-label={t('settings.open')}>⚙️</button>
    </div>
    <h1>{t('app.title')}</h1>
    <p>{t('app.subtitle')}</p>
  </div>

  {#if step !== 'settings'}
    <div class="steps">
    <div class="step {step === 'upload' || step === 'google-import' ? 'active' : ''} {bookmarksUploaded ? 'done' : ''}">
      <span class="step-num">1</span> {t('steps.import')}
    </div>
    <div class="step {step === 'bookmarks' ? 'active' : ''} {bookmarksUploaded ? 'done' : ''}">
      <span class="step-num">2</span> {t('steps.selectBookmarks')}
    </div>
    <div class="step {step === 'connect' ? 'active' : ''} {umapConnected ? 'done' : ''}">
      <span class="step-num">3</span> {t('steps.connectUmap')}
    </div>
    <div class="step {step === 'transfer' ? 'active' : ''}">
      <span class="step-num">4</span> {t('steps.transfer')}
    </div>
  </div>
  {/if}

  {#if step === 'settings'}
    <Settings onBack={closeSettings} />
  {:else if step === 'upload'}
    <Upload {onUpload} {onGoogleImport} />
  {:else if step === 'google-import'}
    <GoogleImport onImport={onImportDone} />
  {:else if step === 'bookmarks'}
    <Bookmarks onSelect={onSelect} onPrev={goPrev} />
  {:else if step === 'connect'}
    <ConnectUmap onConnect={onConnect} onPrev={goPrev} />
  {:else if step === 'transfer'}
    <Transfer onDone={onDone} onPrev={goPrev} />
  {/if}

  {#if aboutOpen}
    <About onClose={() => aboutOpen = false} />
  {/if}
</div>

<style>
  .steps {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.55rem;
    margin-bottom: 1rem;
  }

  .hero {
    position: relative;
    padding: clamp(0.75rem, 2.2vw, 1.35rem) clamp(0.9rem, 3vw, 1.7rem);
    overflow: hidden;
    border: 1px solid rgba(36, 79, 60, 0.14);
    border-radius: 1.25rem;
    background:
      linear-gradient(135deg, rgba(255, 250, 240, 0.92), rgba(238, 246, 237, 0.78)),
      radial-gradient(circle at 92% 12%, rgba(217, 116, 43, 0.16), transparent 12rem);
    box-shadow:
      0 16px 42px rgba(36, 79, 60, 0.12),
      inset 0 1px 0 rgba(255, 255, 255, 0.76);
  }

  .hero::after {
    position: absolute;
    right: -2rem;
    bottom: -5.4rem;
    width: 11rem;
    height: 11rem;
    content: '';
    border: 1px solid rgba(36, 79, 60, 0.16);
    border-radius: 40% 60% 54% 46%;
    background: rgba(76, 127, 99, 0.11);
    transform: rotate(-16deg);
  }

  .hero-actions {
    position: absolute;
    top: 0.65rem;
    right: 0.65rem;
    z-index: 1;
    display: flex;
    gap: 0.35rem;
    padding: 0.25rem;
    border: 1px solid rgba(36, 79, 60, 0.14);
    border-radius: 999px;
    background: rgba(255, 252, 244, 0.72);
    box-shadow: 0 10px 28px rgba(36, 79, 60, 0.12);
  }

  .hero-action {
    width: 2.35rem;
    min-height: 2.35rem;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 999px;
    background: transparent;
    box-shadow: none;
    color: #244f3c;
    cursor: pointer;
    font-size: 1.05rem;
  }

  .hero-action:hover {
    border-color: rgba(217, 116, 43, 0.34);
    background: #fffaf0;
    color: #9f4e1a;
  }

  .step {
    display: flex;
    min-height: 2.75rem;
    align-items: center;
    gap: 0.55rem;
    padding: 0.5rem 0.72rem;
    border: 1px solid rgba(36, 79, 60, 0.12);
    border-radius: 1rem;
    background: rgba(255, 252, 244, 0.62);
    color: #6b746d;
    font-size: 0.85rem;
    font-weight: 700;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.78);
  }

  .step.active {
    border-color: rgba(217, 116, 43, 0.34);
    background: #fff8e8;
    color: #9f4e1a;
    box-shadow: 0 12px 28px rgba(217, 116, 43, 0.12);
  }

  .step.done {
    border-color: rgba(55, 109, 82, 0.24);
    background: #edf8ef;
    color: #244f3c;
  }

  .step-num {
    display: inline-flex;
    width: 1.55rem;
    height: 1.55rem;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: rgba(36, 79, 60, 0.11);
    font-size: 0.75rem;
    font-weight: 800;
  }

  .step.active .step-num {
    background: #d9742b;
    color: #fffaf0;
  }

  .step.done .step-num {
    background: #376d52;
    color: #fffaf0;
  }

  :global(.nav-row) {
    display: flex;
    justify-content: space-between;
    margin-top: 1.5rem;
    gap: 0.75rem;
  }

  :global(.nav-prev), :global(.nav-next) {
    padding: 0.65rem 1rem;
    border: 1px solid rgba(36, 79, 60, 0.16);
    border-radius: 999px;
    background: rgba(255, 252, 244, 0.82);
    box-shadow: 0 8px 18px rgba(36, 79, 60, 0.08);
    cursor: pointer;
    font-size: 0.88rem;
    font-weight: 700;
    color: #244f3c;
    transition: all 0.15s;
  }

  :global(.nav-prev:hover), :global(.nav-next:hover) {
    border-color: rgba(217, 116, 43, 0.38);
    color: #9f4e1a;
  }

  :global(.nav-row button.primary) {
    width: auto;
    min-width: min(18rem, 62vw);
    margin-left: auto;
  }

  :global(button.primary) {
    width: 100%;
    justify-content: center;
    padding: 0.82rem 1rem;
    border: none;
    border-radius: 999px;
    background: linear-gradient(135deg, #244f3c, #376d52);
    color: #fffaf0;
    cursor: pointer;
    font-size: 0.95rem;
    font-weight: 800;
    transition: all 0.2s;
  }

  :global(button.primary:hover) {
    background: linear-gradient(135deg, #1f4635, #326449);
  }

  :global(button.primary:disabled) {
    opacity: 0.58;
    cursor: not-allowed;
    transform: none;
  }

  @media (max-width: 720px) {
    .steps {
      grid-template-columns: 1fr 1fr;
    }

    .hero {
      border-radius: 1.25rem;
    }
  }
</style>
