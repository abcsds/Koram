<script>
  import { API } from '../constants.js';
  import { settings } from '../store.svelte.js';
  import { formatCount } from '../utils.js';

  let { person } = $props(); // { id, name, total }

  const checked = $derived(settings.selected.has(person.id));
  const isNamed = $derived(!!person.name);
  const override = $derived(settings.perPersonOverrides[person.id] ?? null);

  function toggle() { settings.toggleSelected(person.id); }

  function setMode(mode) {
    settings.setOverride(person.id, mode === settings.displayMode ? null : mode);
  }
</script>

<div class="row" class:checked>
  <label>
    <input type="checkbox" {checked} onchange={toggle} />
    <img class="avatar" src={API.personThumb(person.id)} alt="" loading="lazy" />
    <span class="name" class:muted={!isNamed}>
      {person.name ?? `(unnamed #${person.id.slice(0, 4)})`}
    </span>
    <span class="count mono muted">{formatCount(person.total ?? 0)}</span>
  </label>
  {#if checked && isNamed}
    <div class="mode">
      <button
        class:active={(override ?? settings.displayMode) === 'thumbnail'}
        onclick={() => setMode('thumbnail')}
        title="Show face thumbnail">thumb</button>
      <button
        class:active={(override ?? settings.displayMode) === 'name'}
        onclick={() => setMode('name')}
        title="Show name only">name</button>
    </div>
  {/if}
</div>

<style>
  .row {
    border-bottom: 1px solid var(--border);
    padding: 8px 12px;
  }
  label {
    display: grid;
    grid-template-columns: 18px 32px 1fr auto;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }
  .avatar {
    width: 32px; height: 32px;
    border-radius: 50%;
    object-fit: cover;
    background: var(--surface);
  }
  .name { font-size: 13px; }
  .count { font-size: 12px; }
  .mode {
    display: flex; gap: 4px;
    margin-left: 60px;
    margin-top: 6px;
  }
  .mode button {
    padding: 2px 8px;
    font-size: 11px;
    text-transform: lowercase;
    background: transparent;
  }
  .mode button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
</style>
