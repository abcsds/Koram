<script>
  let { open = $bindable(false) } = $props();

  function close() { open = false; }
  function onKey(e) { if (e.key === 'Escape') close(); }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div
    class="backdrop"
    onclick={close}
    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') close(); }}
    role="button"
    tabindex="-1"
    aria-label="Close help"
  ></div>
  <div class="modal" role="dialog" aria-modal="true" aria-labelledby="help-title">
    <div class="head">
      <h2 id="help-title">Keybindings &amp; mouse</h2>
      <button class="close" onclick={close} aria-label="Close help">✕</button>
    </div>
    <dl>
      <dt>Drag</dt>
      <dd>Pan the canvas</dd>

      <dt>Ctrl + drag a face</dt>
      <dd>Move that face. Forces stay live during the drag, then resume when you let go.</dd>

      <dt>Scroll / pinch</dt>
      <dd>Zoom</dd>

      <dt>Double-click</dt>
      <dd>Zoom in (centered on the cursor)</dd>

      <dt>Click a face</dt>
      <dd>Highlight it &amp; its neighbors</dd>

      <dt>Click empty space</dt>
      <dd>Clear the highlight (deselect)</dd>

      <dt>Alt + click a face</dt>
      <dd>Open that person in Immich (new tab)</dd>

      <dt>Hover a face</dt>
      <dd>Show name and photo count</dd>

      <dt>Esc</dt>
      <dd>Close this dialog or the settings drawer</dd>
    </dl>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 40;
    border: none;
    cursor: default;
  }
  .modal {
    position: fixed;
    top: 50%; left: 50%;
    transform: translate(-50%, -50%);
    width: min(480px, 92vw);
    max-height: 80vh;
    overflow-y: auto;
    background: rgba(15, 15, 18, 0.98);
    backdrop-filter: blur(20px);
    border: 1px solid var(--border);
    border-radius: 10px;
    z-index: 41;
    padding: 20px 24px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  }
  .head {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 12px;
  }
  h2 { margin: 0; font-size: 15px; font-weight: 500; letter-spacing: 0.3px; }
  .close {
    width: 28px; height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    color: var(--foreground);
    cursor: pointer;
    border-radius: 4px;
  }
  .close:hover { background: var(--surface); }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px 18px;
    margin: 0;
    font-size: 13px;
  }
  dt {
    font-family: var(--font-mono);
    color: var(--accent);
    font-size: 12px;
    align-self: start;
    padding-top: 1px;
  }
  dd {
    margin: 0;
    color: var(--foreground);
  }
</style>
