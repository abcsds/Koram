<script>
  import { onMount } from 'svelte';
  import TopBar from './lib/components/TopBar.svelte';
  import SettingsDrawer from './lib/components/SettingsDrawer.svelte';
  import GraphCanvas from './lib/components/GraphCanvas.svelte';
  import ExportFab from './lib/components/ExportFab.svelte';
  import { settings, graphState } from './lib/store.svelte.js';
  import { hydrate, trackPersistence } from './lib/persistence.svelte.js';
  import { getPeople, getConnection, openProgressSocket } from './lib/api.js';
  import { API } from './lib/constants.js';

  let people = $state([]);
  let loadError = $state(null);
  let connectionError = $state(null);   // for the 401/unreachable banner
  let canvasComp = $state(null);
  let canvasWrapEl = $state(null);
  let lastJobKey = $state(null);

  hydrate(settings);
  trackPersistence(settings);

  // WS message handler — drives graphState lifecycle.
  // The HTTP /api/graph/compute now returns immediately; we wait for WS terminal status.
  function onProgress(p) {
    graphState.progress = {
      processed: p.processed,
      total: p.total,
      currentPersonName: p.current_person_name ?? null,
    };
    if (p.status === 'running') {
      graphState.status = 'computing';
      graphState.error = null;
    } else if (p.status === 'completed') {
      // `message` carries the cache key for *this* job. Strict equality with the
      // job we kicked off (lastJobKey) guards against stale terminal messages
      // from a previously-spawned background task delivering after we cancelled.
      const key = p.message;
      if (!key || key !== lastJobKey) return;
      fetch(API.graphResult(key))
        .then(r => r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`)))
        .then(result => {
          graphState.result = result;
          graphState.status = 'ready';
        })
        .catch(e => { graphState.error = e.message; graphState.status = 'error'; });
    } else if (p.status === 'cancelled') {
      graphState.status = 'idle';
      lastJobKey = null;
    } else if (p.status === 'error') {
      graphState.error = p.message ?? 'Compute failed';
      graphState.status = 'error';
      lastJobKey = null;
    }
  }

  onMount(async () => {
    try {
      const conn = await getConnection();
      if (!conn.ok) connectionError = conn.error ?? 'Immich is unreachable.';
    } catch (e) {
      connectionError = e.message;
    }

    if (!connectionError) {
      try {
        people = await getPeople();
      } catch (e) {
        loadError = e.message;
      }
    }

    const ws = openProgressSocket(onProgress);
    return () => ws.close();
  });

  function sceneSnapshot() {
    const r = graphState.result;
    if (!r || !canvasComp?.getScene) return { width: 0, height: 0, nodes: [], edges: [] };
    return canvasComp.getScene();
  }

  function toggleDrawer() { settings.drawerOpen = !settings.drawerOpen; }

  // Click-outside closes the drawer (spec line 339).
  function onCanvasMouseDown() {
    if (settings.drawerOpen) settings.drawerOpen = false;
  }

  // Expose the chosen job key so the WS handler can call /result with it.
  // SettingsDrawer sets this when the compute kicks off.
  function setLastJobKey(k) { lastJobKey = k; }

  const isEmpty = $derived(!graphState.result && graphState.status !== 'computing' && !graphState.error);

  // No-edges-after-compute overlay (spec line 432)
  const computedButEmpty = $derived(
    graphState.result && graphState.result.pairs.filter(p => p.count >= settings.minEdgeWeight).length === 0
  );
</script>

<TopBar onToggleDrawer={toggleDrawer} canvasContainer={canvasWrapEl} />

{#if connectionError}
  <div class="banner">
    Can't reach Immich. Check API key in <code>/app/config/koram.toml</code>.
    <span class="muted">({connectionError})</span>
  </div>
{/if}

<div class="canvas-wrap" bind:this={canvasWrapEl} onmousedown={onCanvasMouseDown}>
  {#if graphState.result && !computedButEmpty}
    <GraphCanvas bind:this={canvasComp} />
  {:else if computedButEmpty}
    <div class="centered hint">
      <p>No co-occurrences. Lower min weight or pick more people.</p>
      <button onclick={toggleDrawer}>Open settings</button>
    </div>
  {:else if loadError}
    <div class="centered err">Couldn't load people: {loadError}</div>
  {:else if graphState.error}
    <div class="centered err">{graphState.error}</div>
  {:else if isEmpty}
    <div class="centered hint">
      <p>Pick at least 2 people from the menu.</p>
      <button onclick={toggleDrawer}>Open settings</button>
    </div>
  {:else}
    <div class="centered">Computing…</div>
  {/if}
</div>

<SettingsDrawer {people} {setLastJobKey} />
<ExportFab {sceneSnapshot} />

<style>
  .canvas-wrap {
    position: fixed; inset: 0;
    /* Subtle accent wash so the canvas doesn't read as a flat black void. Matches the
       landing-page hero: purple from the upper-right, cyan from the lower-left. The
       graph canvas clears to transparent, so this shows through between nodes. */
    background:
      radial-gradient(1100px 800px at 85% -5%, rgba(124, 58, 237, 0.18), transparent 70%),
      radial-gradient(900px 700px at -5% 30%, rgba(8, 145, 178, 0.14), transparent 70%),
      var(--bg-deep);
  }
  .banner {
    position: fixed; top: 44px; left: 0; right: 0;
    background: rgba(220, 38, 38, 0.15);
    border-bottom: 1px solid var(--destructive);
    color: var(--foreground);
    padding: 8px 16px;
    font-size: 13px;
    z-index: 15;
  }
  .banner code { font-family: var(--font-mono); background: rgba(0,0,0,0.4); padding: 1px 6px; border-radius: 4px; }
  .centered {
    position: absolute; inset: 0;
    display: grid; place-items: center;
    text-align: center;
    color: var(--foreground-muted);
  }
  .centered.err { color: var(--destructive); }
  .centered button { margin-top: 12px; }
</style>
