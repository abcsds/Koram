<script>
  import { settings } from '../store.svelte.js';

  let { selectedPeople } = $props();   // Array of currently selected person objects (for the unnamed-disable check)

  const anyUnnamed = $derived(selectedPeople.some(p => !p.name));

  function setMode(m) {
    if (m === 'name' && anyUnnamed) return;
    settings.displayMode = m;
  }
</script>

<fieldset>
  <legend>Display</legend>

  <div class="group">
    <span class="label">Node style</span>
    <label><input type="radio" name="ns" value="thumbnail"
      checked={settings.displayMode === 'thumbnail'}
      onchange={() => setMode('thumbnail')} /> Face thumbnail</label>
    <label class:disabled={anyUnnamed} title={anyUnnamed ? 'Disabled: some selected people are unnamed' : ''}>
      <input type="radio" name="ns" value="name"
        checked={settings.displayMode === 'name'}
        onchange={() => setMode('name')}
        disabled={anyUnnamed} /> Name only
    </label>
  </div>

  <div class="group">
    <span class="label">Edge weight</span>
    <label><input type="radio" name="ew" value="count"
      checked={settings.edgeMode === 'count'}
      onchange={() => settings.edgeMode = 'count'} /> Photo count</label>
    <label><input type="radio" name="ew" value="jaccard"
      checked={settings.edgeMode === 'jaccard'}
      onchange={() => settings.edgeMode = 'jaccard'} /> Jaccard similarity</label>
  </div>

  <div class="group">
    {#if settings.edgeMode === 'count'}
      <label class="label-inline">
        Min photos
        <input type="number" min="1" step="1" bind:value={settings.minEdgeWeight} />
      </label>
    {:else}
      <label class="label-inline">
        Min similarity
        <input type="range" min="0" max="1" step="0.01" bind:value={settings.minEdgeWeight} />
        <span class="mono">{Number(settings.minEdgeWeight).toFixed(2)}</span>
      </label>
    {/if}
  </div>
</fieldset>

<style>
  fieldset { border: 1px solid var(--border); border-radius: var(--radius-control); padding: 10px 12px; margin: 0; }
  legend { padding: 0 6px; font-size: 12px; color: var(--foreground-muted); text-transform: uppercase; letter-spacing: 0.5px; }
  .group { margin-top: 8px; display: flex; flex-direction: column; gap: 4px; }
  .label { font-size: 12px; color: var(--foreground-muted); }
  .label-inline { display: flex; gap: 8px; align-items: center; font-size: 13px; }
  label { font-size: 13px; display: flex; gap: 6px; align-items: center; }
  label.disabled { color: var(--foreground-muted); cursor: not-allowed; }
  input[type="number"] { width: 70px; }
  input[type="range"] { flex: 1; }
</style>
