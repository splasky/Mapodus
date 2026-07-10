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
  function goNext() {
    switch (step) {
      case 'upload':
      case 'google-import':
        if (bookmarksUploaded) step = 'bookmarks';
        break;
      case 'connect':
        if (umapConnected) step = 'transfer';
        break;
    }
  }
</script>

<div class="layout">
  <div class="hero">
    <button class="settings-button" onclick={openSettings} aria-label={t('settings.open')}>⚙️</button>
    <button class="about-button" onclick={openAbout} aria-label={t('about.open')}>ℹ️</button>
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
    <ConnectUmap onConnect={onConnect} onPrev={goPrev} onNext={goNext} />
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
    gap: 0.65rem;
    margin-bottom: 1.35rem;
  }

  .hero {
    position: relative;
    padding: clamp(1.25rem, 4vw, 2.75rem) clamp(1rem, 4vw, 2.25rem);
    overflow: hidden;
    border: 1px solid rgba(36, 79, 60, 0.14);
    border-radius: 1.8rem;
    background:
      linear-gradient(135deg, rgba(255, 250, 240, 0.92), rgba(238, 246, 237, 0.78)),
      radial-gradient(circle at 92% 12%, rgba(217, 116, 43, 0.18), transparent 16rem);
    box-shadow:
      0 24px 70px rgba(36, 79, 60, 0.14),
      inset 0 1px 0 rgba(255, 255, 255, 0.76);
  }

  .hero::after {
    position: absolute;
    right: -2.5rem;
    bottom: -4.8rem;
    width: 14rem;
    height: 14rem;
    content: '';
    border: 1px solid rgba(36, 79, 60, 0.16);
    border-radius: 40% 60% 54% 46%;
    background: rgba(76, 127, 99, 0.11);
    transform: rotate(-16deg);
  }

  .settings-button {
    position: absolute;
    top: 1rem;
    right: 1rem;
    z-index: 1;
    width: auto;
    min-height: 2.75rem;
    border: 1px solid rgba(36, 79, 60, 0.16);
    border-radius: 999px;
    background: rgba(255, 252, 244, 0.86);
    box-shadow: 0 10px 28px rgba(36, 79, 60, 0.12);
    color: #244f3c;
    cursor: pointer;
    font-size: 1.1rem;
    padding: 0.45rem 0.72rem;
  }

  .settings-button:hover {
    border-color: rgba(217, 116, 43, 0.42);
    color: #9f4e1a;
  }

  .about-button {
    position: absolute;
    top: 1rem;
    right: 4rem;
    z-index: 1;
    width: auto;
    min-height: 2.75rem;
    border: 1px solid rgba(36, 79, 60, 0.16);
    border-radius: 999px;
    background: rgba(255, 252, 244, 0.86);
    box-shadow: 0 10px 28px rgba(36, 79, 60, 0.12);
    color: #244f3c;
    cursor: pointer;
    font-size: 1.1rem;
    padding: 0.45rem 0.72rem;
  }

  .about-button:hover {
    border-color: rgba(217, 116, 43, 0.42);
    color: #9f4e1a;
  }

  .settings-button:hover {
    border-color: rgba(217, 116, 43, 0.42);
    color: #9f4e1a;
  }

  .step {
    display: flex;
    min-height: 3.2rem;
    align-items: center;
    gap: 0.55rem;
    padding: 0.62rem 0.82rem;
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
