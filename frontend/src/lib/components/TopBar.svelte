<script>
  import ConnectionStatus from './ConnectionStatus.svelte';
  import HelpDialog from './HelpDialog.svelte';
  import { graphState, settings } from '../store.svelte.js';
  import { formatCount } from '../utils.js';

  let { onToggleDrawer, canvasContainer = null } = $props();

  let visible = $state(true);
  let hideTimer = null;
  let helpOpen = $state(false);
  function toggleHelp() { helpOpen = !helpOpen; }

  function bump() {
    visible = true;
    if (hideTimer) clearTimeout(hideTimer);
    hideTimer = setTimeout(() => { visible = false; }, 3000);
  }

  $effect(() => {
    bump();
    // Spec: "Auto-hides after 3s of cursor inactivity over the canvas".
    // Bind to the canvas container if provided, else fall back to window.
    const target = canvasContainer ?? window;
    target.addEventListener('mousemove', bump);
    window.addEventListener('keydown', bump);
    return () => {
      target.removeEventListener('mousemove', bump);
      window.removeEventListener('keydown', bump);
      if (hideTimer) clearTimeout(hideTimer);
    };
  });

  const computing = $derived(graphState.status === 'computing');

  const counter = $derived.by(() => {
    if (computing) {
      const { processed, total, currentPersonName } = graphState.progress;
      return `Fetching · ${currentPersonName ?? '…'} · ${processed}/${total}`;
    }
    if (!graphState.result) return '';
    const nodes = graphState.result.people.length;
    const edges = graphState.result.pairs.filter(p => p.count >= settings.minEdgeWeight).length;
    return `${formatCount(nodes)} nodes · ${formatCount(edges)} edges`;
  });

  const progressPct = $derived.by(() => {
    if (!computing) return 0;
    const { processed, total } = graphState.progress;
    if (!total) return 0;
    return Math.min(100, (processed / total) * 100);
  });
</script>

<div class="bar" class:visible aria-hidden={!visible}>
  <button class="hamburger" onclick={onToggleDrawer} aria-label="Open settings">
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5">
      <line x1="3" y1="6"  x2="17" y2="6"  />
      <line x1="3" y1="10" x2="17" y2="10" />
      <line x1="3" y1="14" x2="17" y2="14" />
    </svg>
  </button>
  <div class="counter mono">{counter}</div>
  <button class="help" onclick={toggleHelp} aria-label="Show keybindings" title="Keybindings (?)">?</button>
  <div class="status"><ConnectionStatus /></div>

  {#if computing}
    <div class="progress" aria-hidden="true">
      <div class="progress-fill" style:width="{progressPct}%"></div>
    </div>
  {/if}
</div>

<HelpDialog bind:open={helpOpen} />

<style>
  .bar {
    position: fixed; top: 0; left: 0; right: 0;
    height: 44px;
    display: flex; align-items: center;
    padding: 0 12px;
    gap: 12px;
    background: rgba(10, 10, 12, 0.6);
    backdrop-filter: blur(20px);
    border-bottom: 1px solid var(--border);
    z-index: 10;
    transition: opacity 250ms var(--easing), transform 250ms var(--easing);
    opacity: 0;
    transform: translateY(-8px);
    pointer-events: none;
  }
  .bar.visible { opacity: 1; transform: translateY(0); pointer-events: auto; }
  .bar:focus-within { opacity: 1; transform: translateY(0); pointer-events: auto; }

  .hamburger {
    width: 36px; height: 36px;
    display: grid; place-items: center;
    background: transparent;
    border: none;
    color: var(--foreground);
  }
  .hamburger:hover { background: var(--surface); }

  .help {
    width: 28px; height: 28px;
    padding: 0;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 50%;
    color: var(--foreground-muted);
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1;
    cursor: pointer;
  }
  .help:hover { background: var(--surface); color: var(--foreground); }

  .counter {
    flex: 1;
    text-align: center;
    font-size: 12px;
    color: var(--foreground-muted);
  }

  .status { display: flex; align-items: center; }

  .progress {
    position: absolute;
    left: 0; right: 0; bottom: -1px;
    height: 2px;
    background: transparent;
  }
  .progress-fill {
    height: 100%;
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent-glow);
    transition: width 200ms var(--easing);
  }
</style>
