<script>
  import DateRange from './DateRange.svelte';
  import DisplayControls from './DisplayControls.svelte';
  import PeopleList from './PeopleList.svelte';
  import { settings, graphState } from '../store.svelte.js';
  import { computeGraph, cancelGraph } from '../api.js';

  let { people, setLastJobKey } = $props();   // all people from /api/people; lifecycle hook from App

  const selectedPeople = $derived(people.filter(p => settings.selected.has(p.id)));
  const peopleIdSet = $derived(new Set(people.map(p => p.id)));

  // Cached people that are no longer in /api/people (merged or deleted in Immich).
  const missingFromImmich = $derived.by(() => {
    if (!graphState.result) return [];
    return graphState.result.people
      .map(p => p.id)
      .filter(id => !peopleIdSet.has(id));
  });

  const isStale = $derived.by(() => {
    if (!graphState.result) return false;
    if (missingFromImmich.length > 0) return true;
    const r = graphState.result;
    const ids = [...settings.selected].sort().join(',');
    const cachedIds = r.people.map(p => p.id).sort().join(',');
    return ids !== cachedIds || (r.from ?? '') !== settings.dateFrom || (r.to ?? '') !== settings.dateTo;
  });

  const staleReason = $derived.by(() => {
    if (missingFromImmich.length > 0) return 'Some people were removed from Immich';
    if (isStale) return 'Settings changed since last refresh';
    return '';
  });

  async function onRefresh() {
    if (graphState.status === 'computing') {
      await cancelGraph();
      return;
    }
    if (settings.selected.size === 0) return;
    graphState.error = null;
    graphState.status = 'computing';
    try {
      const res = await computeGraph({
        person_ids: [...settings.selected],
        from: settings.dateFrom || null,
        to: settings.dateTo || null,
        force: true,
      });
      // The HTTP response returns immediately; the actual result lands via WS → App.svelte.
      // Only short-circuit when the backend served from cache.
      setLastJobKey?.(res.key);
      if (res.cached && res.result) {
        graphState.result = res.result;
        graphState.status = 'ready';
      }
    } catch (e) {
      graphState.error = e.message;
      graphState.status = 'error';
    }
  }

  function onClose() { settings.drawerOpen = false; }

  function onKey(e) { if (e.key === 'Escape') settings.drawerOpen = false; }
</script>

<svelte:window onkeydown={onKey} />

{#if settings.drawerOpen}
  <aside class="drawer">
    <div class="head">
      <h2>Settings</h2>
      <button class="close" onclick={onClose} aria-label="Close settings">✕</button>
    </div>

    <div class="body">
      <DateRange />
      <DisplayControls {selectedPeople} />

      <fieldset class="people">
        <legend>People</legend>
        <PeopleList {people} />
      </fieldset>
    </div>

    <div class="foot">
      <button class="primary" onclick={onRefresh} title={staleReason}>
        {#if graphState.status === 'computing'}
          Cancel
        {:else if isStale}
          <span class="stale-dot" aria-hidden="true"></span>
          ↻ Refresh ({staleReason || 'stale'})
        {:else}
          ↻ Refresh from Immich
        {/if}
      </button>
    </div>
  </aside>
{/if}

<style>
  .drawer {
    position: fixed; top: 0; left: 0; bottom: 0;
    width: 380px;
    display: flex; flex-direction: column;
    background: rgba(10, 10, 12, 0.82);
    backdrop-filter: blur(20px);
    border-right: 1px solid var(--border);
    z-index: 20;
    animation: slide 250ms var(--easing);
  }
  @keyframes slide { from { transform: translateX(-20px); opacity: 0; } to { transform: none; opacity: 1; } }

  .head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  .head h2 { margin: 0; font-size: 14px; font-weight: 500; letter-spacing: 0.3px; }
  .close { width: 28px; height: 28px; padding: 0; background: transparent; border: none; }
  .close:hover { background: var(--surface); }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex; flex-direction: column; gap: 12px;
  }
  .people { padding: 0; }
  .people legend { padding: 0 6px; margin-left: 8px; font-size: 12px; color: var(--foreground-muted); text-transform: uppercase; letter-spacing: 0.5px; }

  .foot { border-top: 1px solid var(--border); padding: 12px; }
  .foot .primary { width: 100%; display: inline-flex; align-items: center; justify-content: center; gap: 8px; }
  .stale-dot {
    display: inline-block;
    width: 8px; height: 8px;
    border-radius: 50%;
    background: var(--warning);
    box-shadow: 0 0 6px var(--warning);
  }
</style>
