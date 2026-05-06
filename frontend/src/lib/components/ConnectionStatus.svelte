<script>
  import { getConnection } from '../api.js';

  let { } = $props();

  let status = $state('checking');   // 'checking' | 'ok' | 'error'
  let version = $state(null);
  let error = $state(null);

  async function poll() {
    try {
      const r = await getConnection();
      status = r.ok ? 'ok' : 'error';
      version = r.immich_version;
      error = r.error;
    } catch (e) {
      status = 'error';
      error = e.message;
    }
  }

  $effect(() => {
    poll();
    const id = setInterval(poll, 30_000);
    return () => clearInterval(id);
  });
</script>

<span class="dot {status}" title={status === 'ok' ? `Immich ${version}` : (error ?? 'Checking…')}></span>

<style>
  .dot {
    display: inline-block;
    width: 8px; height: 8px;
    border-radius: 50%;
    background: var(--foreground-muted);
    transition: background 200ms var(--easing);
  }
  .dot.ok { background: #22C55E; }
  .dot.error { background: var(--destructive); }
  .dot.checking { background: var(--warning); }
</style>
