<script>
  import { exportPng } from '../graph/png-export.js';
  import { jaccard } from '../graph/jaccard.js';
  import { settings, graphState } from '../store.js';
  import { uploadToImmich } from '../api.js';
  import { downloadBlob } from '../utils.js';

  let { sceneSnapshot } = $props();   // () => { width, height, nodes, edges, displayMode, label, imageCache, colors }

  let open = $state(false);
  let toast = $state(null);
  let busy = $state(false);

  function showToast(msg, kind = 'ok') {
    toast = { msg, kind };
    setTimeout(() => { toast = null; }, 1500);
  }

  async function onPng() {
    if (busy) return;
    busy = true;
    try {
      const blob = await exportPng(sceneSnapshot());
      downloadBlob(blob, `koram-${Date.now()}.png`);
      showToast('PNG saved');
    } catch (e) { showToast(e.message, 'err'); }
    finally { busy = false; open = false; }
  }

  async function onImmich() {
    if (busy) return;
    busy = true;
    try {
      const blob = await exportPng(sceneSnapshot());
      const id = `koram-${Date.now()}`;
      await uploadToImmich(blob, id);
      showToast('Uploaded to Immich');
    } catch (e) {
      showToast(e.message, 'err');
      // Fallback: also offer the local download
      try {
        const blob = await exportPng(sceneSnapshot());
        downloadBlob(blob, `koram-${Date.now()}.png`);
      } catch {}
    }
    finally { busy = false; open = false; }
  }

  function onCsv() {
    if (!graphState.result) return;
    const r = graphState.result;
    const peopleById = new Map(r.people.map(p => [p.id, p]));
    const lines = ['person_a_id,person_a_name,person_b_id,person_b_name,photo_count,jaccard'];
    for (const pair of r.pairs) {
      const a = peopleById.get(pair.a);
      const b = peopleById.get(pair.b);
      if (!a || !b) continue;
      const j = jaccard(pair.count, a.total, b.total);
      const wDisp = settings.edgeMode === 'jaccard' ? j : pair.count;
      if (wDisp < settings.minEdgeWeight) continue;
      const cells = [
        pair.a, csvEscape(a.name ?? ''),
        pair.b, csvEscape(b.name ?? ''),
        String(pair.count), j.toFixed(4),
      ];
      lines.push(cells.join(','));
    }
    const blob = new Blob([lines.join('\n')], { type: 'text/csv' });
    downloadBlob(blob, `koram-${Date.now()}.csv`);
    showToast('CSV saved');
    open = false;
  }

  function csvEscape(s) {
    if (/[",\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
    return s;
  }

  function onKey(e) { if (e.key === 'Escape') open = false; }
</script>

<svelte:window onkeydown={onKey} />

<div class="fab" class:open>
  {#if open}
    <button class="action" onclick={onPng}      disabled={busy}>⤓ PNG</button>
    <button class="action" onclick={onCsv}      disabled={busy}>⤓ CSV</button>
    <button class="action" onclick={onImmich}   disabled={busy}>⇪ Immich</button>
  {/if}
  <button class="trigger" onclick={() => open = !open} aria-label="Export">
    {open ? '✕' : '⤓'}
  </button>
</div>

{#if toast}
  <div class="toast {toast.kind}">{toast.msg}</div>
{/if}

<style>
  .fab {
    position: fixed; right: 20px; bottom: 20px;
    display: flex; flex-direction: column; gap: 8px; align-items: flex-end;
    z-index: 25;
  }
  .trigger, .action {
    width: 48px; height: 48px;
    border-radius: 50%;
    background: rgba(10, 10, 12, 0.82);
    backdrop-filter: blur(20px);
    border: 1px solid var(--border);
    color: var(--foreground);
    font-size: 16px;
    box-shadow: 0 6px 20px rgba(0,0,0,0.4);
    transition: transform 200ms var(--easing);
  }
  .action {
    width: auto;
    height: 40px;
    border-radius: 20px;
    padding: 0 14px;
    font-size: 13px;
  }
  .trigger:hover, .action:hover { transform: translateY(-1px); }

  .toast {
    position: fixed; bottom: 88px; right: 20px;
    padding: 8px 14px;
    background: rgba(10, 10, 12, 0.92);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
    z-index: 26;
  }
  .toast.err { border-color: var(--destructive); color: var(--destructive); }
</style>
