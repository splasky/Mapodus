<script lang="ts">
  import { apiPost } from '../api';
  import { t } from '../i18n';

  let { onClose }: { onClose: () => void } = $props();

  const version = '0.1.0';
  const repoUrl = 'https://github.com/splasky/Mapodus';
  const releasesUrl = 'https://github.com/splasky/Mapodus/releases';
  const issueUrl = 'https://github.com/splasky/Mapodus/issues';
  const creditsUrl = 'https://github.com/splasky';

  let latestVersion = $state(version);
  let releaseNote = $state('');
  let checkingUpdate = $state(false);

  // About opens as a popup, so check GitHub releases when the popup is shown.
  $effect(() => {
    checkForUpdates();
  });

  async function checkForUpdates() {
    checkingUpdate = true;
    try {
      const response = await fetch('https://api.github.com/repos/splasky/Mapodus/releases/latest');
      if (!response.ok) {
        return;
      }
      const data = await response.json();
      latestVersion = data.tag_name || data.name || '';
      releaseNote = data.name || data.tag_name || '';
    } catch {
      // Keep the current app version visible when release checks are unavailable.
    } finally {
      checkingUpdate = false;
    }
  }

  function normalizeVersion(value: string) {
    return value.trim().replace(/^v/i, '');
  }

  async function openExternal(url: string) {
    try {
      await apiPost('/open-external', { url });
    } catch {
      window.open(url, '_blank', 'noopener,noreferrer');
    }
  }

  function closeFromBackdrop(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onClose();
    }
  }

  function closeOnEscape(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      onClose();
    }
  }
</script>

<div class="about-backdrop" role="presentation" onclick={closeFromBackdrop}>
  <div class="about-dialog card" role="dialog" aria-modal="true" aria-labelledby="about-title" tabindex="-1" onkeydown={closeOnEscape}>
    <button class="close-button" onclick={onClose} aria-label={t('about.close')}>x</button>

    <div class="about-heading">
      <div class="app-icon" aria-hidden="true">M</div>
      <div>
        <h2 id="about-title">{t('about.title')}</h2>
        <p>{t('about.subtitle')}</p>
      </div>
    </div>

    <dl class="about-list">
      <div>
        <dt>{t('about.versionLabel')}</dt>
        <dd>{version}</dd>
      </div>
      <div>
        <dt>{t('about.latestReleaseLabel')}</dt>
        <dd>
          {#if checkingUpdate}
            {t('about.checking')}
          {:else}
            {latestVersion || version}
            {#if latestVersion && normalizeVersion(latestVersion) !== version}
              <span class="update-note">{t('about.updateAvailable')}</span>
            {/if}
          {/if}
        </dd>
      </div>
      <div>
        <dt>{t('about.releaseNotesLabel')}</dt>
        <dd>
          {#if releaseNote}
            {releaseNote}
          {:else}
            {t('about.releaseNotes')}
          {/if}
          <button class="text-link" onclick={() => openExternal(releasesUrl)}>{t('about.viewReleases')}</button>
        </dd>
      </div>
      <div>
        <dt>{t('about.creditsLabel')}</dt>
        <dd><button class="text-link first-link" onclick={() => openExternal(creditsUrl)}>{t('about.credits')}</button></dd>
      </div>
      <div>
        <dt>{t('about.legalLabel')}</dt>
        <dd>{t('about.legal')}</dd>
      </div>
      <div>
        <dt>{t('about.linksLabel')}</dt>
        <dd class="links">
          <button class="text-link first-link" onclick={() => openExternal(repoUrl)}>{t('about.website')}</button>
          <button class="text-link" onclick={() => openExternal(issueUrl)}>{t('about.reportIssue')}</button>
        </dd>
      </div>
    </dl>
  </div>
</div>

<style>
  .about-backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: rgba(24, 37, 31, 0.34);
    backdrop-filter: blur(8px);
  }

  .about-dialog {
    width: min(28rem, 100%);
    gap: 1.1rem;
    margin: 0;
  }

  .close-button {
    position: absolute;
    top: 0.85rem;
    right: 0.85rem;
    min-height: 2rem;
    padding: 0.2rem 0.72rem;
    font-size: 1.25rem;
    line-height: 1;
  }

  .about-heading {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    padding-right: 2.5rem;
  }

  .about-heading p {
    margin: 0.25rem 0 0;
  }

  .app-icon {
    display: grid;
    width: 3.6rem;
    height: 3.6rem;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 1rem;
    background: linear-gradient(135deg, #244f3c, #d9742b);
    color: #fffaf0;
    font-size: 1.7rem;
    font-weight: 800;
  }

  .about-list {
    display: grid;
    gap: 0;
    margin: 0;
    overflow: hidden;
    border: 1px solid rgba(36, 79, 60, 0.12);
    border-radius: 1rem;
    background: rgba(255, 255, 255, 0.42);
  }

  .about-list div {
    display: grid;
    grid-template-columns: 8rem 1fr;
    gap: 1rem;
    padding: 0.78rem 0.9rem;
    border-bottom: 1px solid rgba(36, 79, 60, 0.1);
  }

  .about-list div:last-child {
    border-bottom: 0;
  }

  dt {
    color: #244f3c;
    font-size: 0.85rem;
    font-weight: 800;
  }

  dd {
    margin: 0;
    color: #526056;
    font-size: 0.9rem;
  }

  .text-link {
    display: inline-block;
    min-height: auto;
    margin-left: 0.45rem;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    color: #9f4e1a;
    font-weight: 700;
    text-decoration: underline;
    text-decoration-color: rgba(159, 78, 26, 0.28);
    text-decoration-thickness: 0.12em;
    text-underline-offset: 0.18em;
  }

  .text-link:hover {
    background: transparent;
    box-shadow: none;
    transform: none;
  }

  .first-link {
    margin-left: 0;
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }


  .update-note {
    display: inline-block;
    margin-left: 0.45rem;
    color: #9f4e1a;
    font-weight: 800;
  }

  @media (max-width: 560px) {
    .about-list div {
      grid-template-columns: 1fr;
      gap: 0.2rem;
    }

    .text-link {
      margin-right: 0.45rem;
      margin-left: 0;
    }
  }
</style>
