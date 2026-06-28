<script lang="ts">
  import { apiGet } from '../api';

  let { onLogin }: { onLogin: () => void } = $props();
  let error = $state('');

  async function checkStatus() {
    try {
      const status = await apiGet<any>('/auth/status');
      if (status.logged_in) {
        onLogin();
      } else {
        window.location.href = '/api/auth/google';
      }
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="card">
  <h2>Sign In</h2>
  <p>Sign in with Google to start converting your saved places.</p>

  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  <button onclick={checkStatus}>Sign in with Google</button>
</div>
