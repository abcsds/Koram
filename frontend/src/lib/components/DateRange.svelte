<script>
  import { settings } from '../store.svelte.js';

  let allTime = $state(!settings.dateFrom && !settings.dateTo);

  function onAllTimeChange() {
    if (allTime) {
      settings.dateFrom = '';
      settings.dateTo = '';
    }
  }
</script>

<fieldset>
  <legend>Date range</legend>
  <div class="row">
    <label>From <input type="date" bind:value={settings.dateFrom} disabled={allTime} /></label>
    <label>To   <input type="date" bind:value={settings.dateTo}   disabled={allTime} /></label>
  </div>
  <label class="check-line">
    <input type="checkbox" bind:checked={allTime} onchange={onAllTimeChange} />
    All time
  </label>
</fieldset>

<style>
  fieldset { border: 1px solid var(--border); border-radius: var(--radius-control); padding: 10px 12px; margin: 0; }
  legend { padding: 0 6px; font-size: 12px; color: var(--foreground-muted); text-transform: uppercase; letter-spacing: 0.5px; }
  .row { display: flex; gap: 10px; flex-wrap: wrap; }
  .row label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--foreground-muted); }
  .check-line { display: flex; gap: 8px; align-items: center; margin-top: 8px; font-size: 13px; }
</style>
