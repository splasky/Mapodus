<script lang="ts">
  import { apiGet } from '../api';

  let { onLogin, onNext }: { onLogin: () => void; onNext?: () => void } = $props();
  let error = $state('');
  let checking = $state(true);

  $effect(() => {
    apiGet<any>('/auth/status')
      .then(status => {
        if (status.logged_in) {
          onLogin();
        }
      })
      .catch(e => error = String(e))
      .finally(() => checking = false);
  });

  function startLogin() {
    window.location.href = '/api/auth/google';
  }
</script>

{#if !checking}
  <div class="card">
    <h2>Sign In</h2>
    <p>Sign in with Google to start converting your saved places.</p>

    {#if error}
      <div class="notice error">{error}</div>
    {/if}

    <button class="primary" onclick={startLogin}>Sign in with Google</button>

    <div class="nav-row">
      <span></span>
      <button class="nav-next" onclick={onNext}>Next</button>
    </div>
  </div>
{/if}
