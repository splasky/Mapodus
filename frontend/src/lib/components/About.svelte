<script lang="ts">
  import { t } from '../i18n';

  let { onBack }: { onBack: () => void } = $props();

  const version = '0.1.0';
  const repoUrl = 'https://github.com/splasky/Gmap-to-uMap';
  const releasesUrl = 'https://github.com/splasky/Gmap-to-uMap/releases';
  const issueUrl = 'https://github.com/splasky/Gmap-to-uMap/issues';
  const creditsUrl = 'https://github.com/splasky';

  let latestVersion = $state('');
  let checkingUpdate = $state(false);
  let updateAvailable = $state(false);
  let releases: Array<{ name: string; body: string }> = $state([]);

  // Fetch latest release info on component mount
  $effect(() => {
    checkForUpdates();
  });

  async function checkForUpdates() {
    checkingUpdate = true;
    try {
      const response = await fetch('https://api.github.com/repos/splasky/Gmap-to-uMap/releases/latest');
      if (response.ok) {
        const data = await response.json();
        latestVersion = data.tag_name || data.name;
        updateAvailable = latestVersion && latestVersion !== `v${version}`;
      }
    } catch (e) {
      // Silently fail if release check fails
      console.debug('Failed to check for updates:', e);
    } finally {
      checkingUpdate = false;
    }
  }

  function openReleases() {
    window.open(releasesUrl, '_blank');
  }
</script>

<div class="card">
  <h2>{t('about.title')}</h2>
  <p class="subtitle">{t('about.subtitle')}</p>

  <div class="about-content">
    <div class="section">
      <h3>{t('about.versionLabel')}</h3>
      <div class="version-info">
        <p>{version}</p>
        <button 
          class="check-update-btn" 
          onclick={checkForUpdates}
          disabled={checkingUpdate}
          aria-label={t('about.checkUpdate')}
        >
          {checkingUpdate ? t('about.checking') : t('about.checkUpdate')}
        </button>
      </div>
      {#if updateAvailable}
        <p class="update-available">{t('about.updateAvailable')} {latestVersion}</p>
      {/if}
    </div>

    <div class="section">
      <h3>{t('about.whatsNewLabel')}</h3>
      <button class="view-releases-btn" onclick={openReleases}>
        {t('about.viewReleases')}
      </button>
    </div>

    <div class="section">
      <h3>{t('about.creditsLabel')}</h3>
      <p>
        <a href={creditsUrl} target="_blank" rel="noopener noreferrer">
          {t('about.credits')}
        </a>
      </p>
    </div>

    <div class="section">
      <h3>{t('about.legalLabel')}</h3>
      <p>{t('about.legal')}</p>
    </div>

    <div class="section links">
      <h3>{t('about.linksLabel')}</h3>
      <div class="link-buttons">
        <a href={repoUrl} target="_blank" rel="noopener noreferrer" class="link-button">
          {t('about.website')}
        </a>
        <a href={issueUrl} target="_blank" rel="noopener noreferrer" class="link-button">
          {t('about.reportIssue')}
        </a>
      </div>
    </div>
  </div>

  <div class="nav-row">
    <button class="nav-prev" onclick={onBack}>{t('common.back')}</button>
  </div>
</div>

<style>
  .subtitle {
    color: #6b746d;
    font-size: 0.95rem;
    margin-top: 0.25rem;
  }

  .about-content {
    margin: 1.5rem 0;
  }

  .section {
    margin-bottom: 1.5rem;
  }

  .section h3 {
    font-size: 0.9rem;
    font-weight: 700;
    color: #244f3c;
    margin-bottom: 0.4rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .section p {
    color: #475569;
    font-size: 0.9rem;
    line-height: 1.5;
    margin: 0;
  }

  .section a {
    color: #d9742b;
    text-decoration: none;
    font-weight: 600;
    transition: color 0.2s;
  }

  .section a:hover {
    color: #9f4e1a;
    text-decoration: underline;
  }

  .version-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .version-info p {
    margin: 0;
  }

  .check-update-btn {
    padding: 0.35rem 0.75rem;
    border: 1px solid rgba(217, 116, 43, 0.3);
    border-radius: 0.4rem;
    background: rgba(217, 116, 43, 0.08);
    color: #d9742b;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .check-update-btn:hover:not(:disabled) {
    border-color: rgba(217, 116, 43, 0.5);
    background: rgba(217, 116, 43, 0.12);
    color: #9f4e1a;
  }

  .check-update-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .update-available {
    margin-top: 0.5rem;
    color: #d9742b;
    font-size: 0.85rem;
    font-weight: 600;
  }

  .view-releases-btn {
    padding: 0.5rem 1rem;
    border: 1px solid rgba(217, 116, 43, 0.3);
    border-radius: 0.5rem;
    background: rgba(217, 116, 43, 0.08);
    color: #d9742b;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .view-releases-btn:hover {
    border-color: rgba(217, 116, 43, 0.5);
    background: rgba(217, 116, 43, 0.12);
    color: #9f4e1a;
  }

  .links .link-buttons {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .link-button {
    display: inline-block;
    padding: 0.5rem 1rem;
    border: 1px solid rgba(217, 116, 43, 0.3);
    border-radius: 0.5rem;
    background: rgba(217, 116, 43, 0.08);
    color: #d9742b;
    font-size: 0.85rem;
    font-weight: 600;
    text-decoration: none;
    transition: all 0.2s;
  }

  .link-button:hover {
    border-color: rgba(217, 116, 43, 0.5);
    background: rgba(217, 116, 43, 0.12);
    color: #9f4e1a;
  }
</style>
