// Platform-aware URL opening utility
// Uses Tauri command on desktop, window.open on web

export async function openUrl(url: string): Promise<void> {
  // Check if we're in desktop mode by checking for Tauri API
  if (typeof window !== 'undefined' && (window as any).__TAURI__) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke<void>('open_url', { url });
      return;
    } catch (e) {
      console.error('Failed to open URL via Tauri:', e);
      // Fallback to window.open
    }
  }

  // Fallback: use window.open for web mode
  window.open(url, '_blank');
}
