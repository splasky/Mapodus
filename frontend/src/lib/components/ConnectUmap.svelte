<script lang="ts">
  import { apiGet, apiPost } from '../api';

  let { onConnect, onPrev, onNext }: { onConnect: () => void; onPrev?: () => void; onNext?: () => void } = $props();
  let umapUrl = $state('https://umap.openstreetmap.fr/en/');
  let username = $state('');
  let password = $state('');
  let passwordSaved = $state(false);
  let connecting = $state(false);
  let error = $state('');

  $effect(() => {
    apiGet<{ umap_default_url: string; umap_account?: string; umap_password_saved: boolean }>('/settings')
      .then(status => {
        if (status.umap_default_url) {
          umapUrl = status.umap_default_url;
        }
        username = status.umap_account ?? '';
        passwordSaved = status.umap_password_saved;
      })
      .catch(() => {
        apiGet<{ umap_url?: string }>('/umap/status')
          .then(status => {
            if (status.umap_url) {
              umapUrl = status.umap_url;
            }
          })
          .catch(() => {
            // Keep the built-in default if the status request fails.
          });
      });
  });

  async function connect() {
    if (!umapUrl || !username || (!password && !passwordSaved)) {
      error = passwordSaved
        ? 'Please fill in uMap URL and username'
        : 'Please fill in all fields';
      return;
    }
    connecting = true;
    error = '';
    try {
      await apiPost('/umap/connect', { umap_url: umapUrl, username, password });
      onConnect();
    } catch (e) {
      error = String(e);
    } finally {
      connecting = false;
    }
  }
</script>

<div class="card">
  <h2>Connect to uMap</h2>
  <p>Enter your uMap instance URL and login credentials.</p>

  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  <label>
    uMap URL
    <input bind:value={umapUrl} placeholder="https://umap.openstreetmap.fr/en/" />
  </label>
  <label>
    Username
    <input bind:value={username} placeholder="your uMap username" />
  </label>
  <label>
    Password
    <input
      type="password"
      bind:value={password}
      placeholder={passwordSaved ? 'Saved password will be used if left blank' : 'your uMap password'}
    />
  </label>

    <button class="primary" onclick={connect} disabled={connecting}>
    {connecting ? 'Connecting...' : 'Connect'}
  </button>

  <div class="nav-row">
    <button class="nav-prev" onclick={onPrev}>Previous</button>
    <button class="nav-next" onclick={onNext}>Next</button>
  </div>
</div>
