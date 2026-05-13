<script>
  import PersonRow from './PersonRow.svelte';
  import { settings } from '../store.svelte.js';

  let { people } = $props();   // Array<{id, name, total}>

  const ROW_HEIGHT = 56;       // 32 avatar + 24 vertical padding/borders

  let containerEl = $state(null);
  let scrollTop = $state(0);
  let viewportH = $state(400);

  function onScroll() { if (containerEl) scrollTop = containerEl.scrollTop; }

  $effect(() => {
    if (!containerEl) return;
    const ro = new ResizeObserver(() => { viewportH = containerEl.clientHeight; });
    ro.observe(containerEl);
    return () => ro.disconnect();
  });

  const filtered = $derived.by(() => {
    const q = settings.search.trim().toLowerCase();
    return people.filter(p => {
      if (!settings.showUnnamed && !p.name) return false;
      if (q && !(p.name ?? '').toLowerCase().includes(q)) return false;
      return true;
    });
  });

  // Keep `settings.selected` in sync with reality. The compute pipeline emits one
  // graph node per selected ID, regardless of whether the face still exists in
  // Immich — so stale IDs render as ghost nodes (label "#xxxx", count 0). Two
  // sources of stale IDs:
  //   1. Faces the user picked, then Immich merged/deleted → not in /api/people
  //      anymore (so even after refresh, the search returns 0 assets for them).
  //   2. Unnamed faces still selected after the user unchecks "Show unnamed".
  // We prune both. Skip the whole pass while `people` is still loading to avoid
  // nuking the selection on first mount.
  $effect(() => {
    if (people.length === 0) return;
    const byId = new Map(people.map(p => [p.id, p]));
    const showUnnamed = settings.showUnnamed;
    const next = [...settings.selected].filter(id => {
      const p = byId.get(id);
      if (!p) return false;                  // gone from Immich
      if (!p.name && !showUnnamed) return false;  // unnamed + hidden
      return true;
    });
    if (next.length !== settings.selected.size) settings.setSelected(next);
  });

  const startIdx = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - 4));
  const endIdx   = $derived(Math.min(filtered.length, Math.ceil((scrollTop + viewportH) / ROW_HEIGHT) + 4));
  const padTop    = $derived(startIdx * ROW_HEIGHT);
  const padBottom = $derived((filtered.length - endIdx) * ROW_HEIGHT);
  const visible   = $derived(filtered.slice(startIdx, endIdx));

  function selectAll()   { settings.setSelected([...settings.selected, ...filtered.map(p => p.id)]); }
  function selectNone()  { const ids = new Set(filtered.map(p => p.id)); settings.setSelected([...settings.selected].filter(id => !ids.has(id))); }
</script>

<div class="header">
  <div class="search-row">
    <input type="text" placeholder="🔍 Search…" bind:value={settings.search} />
    <span class="count mono muted">{filtered.length}</span>
  </div>
  <label class="check-line">
    <input type="checkbox" bind:checked={settings.showUnnamed} />
    Show unnamed faces
  </label>
  <div class="select-actions">
    <button onclick={selectAll}>Select all</button>
    <button onclick={selectNone}>None</button>
  </div>
</div>

<div class="scroller" bind:this={containerEl} onscroll={onScroll}>
  <div style:height="{padTop}px"></div>
  {#each visible as p (p.id)}
    <PersonRow person={p} />
  {/each}
  <div style:height="{padBottom}px"></div>
</div>

<style>
  .header {
    display: flex; flex-direction: column; gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .search-row { display: flex; gap: 8px; align-items: center; }
  .search-row input { flex: 1; }
  .count { font-size: 12px; }
  .check-line { display: flex; gap: 8px; align-items: center; font-size: 13px; }
  .select-actions { display: flex; gap: 6px; }
  .select-actions button { padding: 4px 10px; font-size: 12px; }

  .scroller {
    flex: 1;
    overflow-y: auto;
    min-height: 200px;
  }
</style>
