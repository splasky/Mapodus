<script lang="ts">
  import { apiPost } from '../api';

  let { onConnect }: { onConnect: () => void } = $props();
  let umapUrl = $state('http://localhost:8000/en/');
  let username = $state('');
  let password = $state('');
  let connecting = $state(false);
  let error = $state('');

  async function connect() {
    if (!umapUrl || !username || !password) {
      error = 'Please fill in all fields';
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
    <input bind:value={umapUrl} placeholder="http://localhost:8000/en/" />
  </label>
  <label>
    Username
    <input bind:value={username} placeholder="your uMap username" />
  </label>
  <label>
    Password
    <input type="password" bind:value={password} placeholder="your uMap password" />
  </label>

  <button onclick={connect} disabled={connecting}>
    {connecting ? 'Connecting...' : 'Connect'}
  </button>
</div>
