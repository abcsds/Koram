<script>
  import { onDestroy, untrack } from 'svelte';
  import { drag } from 'd3-drag';
  import { select } from 'd3-selection';
  import { zoom, zoomIdentity } from 'd3-zoom';
  import { buildSimulation, reweight, computeNodeRadius } from '../graph/force.js';
  import { draw, hitTestEdge } from '../graph/render-canvas.js';
  import { jaccard } from '../graph/jaccard.js';
  import { settings, graphState } from '../store.svelte.js';
  import { API } from '../constants.js';
  import { formatCount } from '../utils.js';

  let canvasEl = $state(null);
  let width = $state(0);
  let height = $state(0);
  let transform = $state({ k: 1, x: 0, y: 0 });
  let hover = $state(null);
  let locked = $state(null);
  let tooltip = $state(null); // { x, y, text }

  let nodes = [];
  let edges = [];
  let adjacency = new Map();
  let imageCache = new Map();
  let simulation = null;

  const cssVar = (name) =>
    getComputedStyle(document.documentElement).getPropertyValue(name).trim();

  // A node is "displayable" if it has a name, OR the user has opted to show
  // unnamed faces. The compute pipeline returns one PersonNode per selected ID
  // regardless of whether the face still exists in Immich, so deleted/merged
  // faces and unnamed faces both surface here as `name == null`.
  function isDisplayable(person) {
    return Boolean(person?.name) || settings.showUnnamed;
  }

  function buildEdges(result) {
    const peopleById = new Map(result.people.map(p => [p.id, p]));
    const idSet = new Set(result.people.filter(isDisplayable).map(p => p.id));
    const out = [];
    for (const pair of result.pairs) {
      if (!idSet.has(pair.a) || !idSet.has(pair.b)) continue;
      const a = peopleById.get(pair.a);
      const b = peopleById.get(pair.b);
      const j = jaccard(pair.count, a.total, b.total);
      const wDisplay = settings.edgeMode === 'jaccard' ? j : pair.count;
      if (wDisplay < settings.minEdgeWeight) continue;
      // Internal weight is scaled when in jaccard mode so the force math stays linear.
      const w = settings.edgeMode === 'jaccard' ? j * 1000 : pair.count;
      out.push({ source: pair.a, target: pair.b, weight: w, displayWeight: wDisplay, count: pair.count });
    }
    return out;
  }

  function rebuildAdjacency(eds) {
    const adj = new Map();
    for (const e of eds) {
      const sId = typeof e.source === 'object' ? e.source.id : e.source;
      const tId = typeof e.target === 'object' ? e.target.id : e.target;
      if (!adj.has(sId)) adj.set(sId, new Set());
      if (!adj.has(tId)) adj.set(tId, new Set());
      adj.get(sId).add(tId);
      adj.get(tId).add(sId);
    }
    return adj;
  }

  /** Full rebuild — called only when the *result* changes (new compute). Position-resetting. */
  function rebuildGraph() {
    const r = graphState.result;
    if (!r) { nodes = []; edges = []; adjacency = new Map(); return; }

    nodes = r.people.filter(isDisplayable).map(p => ({
      id: p.id,
      name: p.name,
      total: p.total,
      radius: computeNodeRadius(p.total),
    }));

    edges = buildEdges(r);
    adjacency = rebuildAdjacency(edges);

    if (simulation) simulation.stop();
    simulation = buildSimulation(nodes, edges, width || 800, height || 600)
      .alpha(1)
      .on('tick', render);

    // Pre-fetch thumbnails into the cache
    for (const n of nodes) {
      if (imageCache.has(n.id)) continue;
      const img = new Image();
      img.crossOrigin = 'anonymous';
      img.src = API.personThumb(n.id);
      imageCache.set(n.id, img);
    }
  }

  /** Edges-only rebind — called when min weight or edge mode changes.
      Preserves node positions; just swaps the link force's input. */
  function rebindEdges() {
    const r = graphState.result;
    if (!r || !simulation) return;
    edges = buildEdges(r);
    adjacency = rebuildAdjacency(edges);
    reweight(edges);
    simulation.force('link').links(edges);
    simulation.alpha(0.3).restart();
  }


  function displayMode(id) {
    const override = settings.perPersonOverrides[id];
    if (override) return override;
    const node = nodes.find(n => n.id === id);
    if (!node?.name) return 'thumbnail';
    return settings.displayMode;
  }

  function label(id) {
    const node = nodes.find(n => n.id === id);
    return node?.name ?? `#${id.slice(0, 4)}`;
  }

  function render() {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext('2d');
    draw(ctx, width, height, {
      nodes, edges, transform, hover, locked, adjacency, imageCache,
      displayMode, label,
      edgeColor: cssVar('--edge') || '#0891B2',
      accentColor: cssVar('--accent') || '#7C3AED',
      accentGlow: cssVar('--accent-glow') || 'rgba(124,58,237,0.20)',
    });
  }

  function onResize() {
    if (!canvasEl) return;
    width = canvasEl.clientWidth;
    height = canvasEl.clientHeight;
    const dpr = window.devicePixelRatio || 1;
    canvasEl.width = Math.floor(width * dpr);
    canvasEl.height = Math.floor(height * dpr);
    if (simulation) {
      simulation.force('center').x(width / 2).y(height / 2);
      simulation.alpha(0.3).restart();
    }
    render();
  }

  function clientToGraph(cx, cy) {
    const rect = canvasEl.getBoundingClientRect();
    const x = (cx - rect.left - transform.x) / transform.k;
    const y = (cy - rect.top  - transform.y) / transform.k;
    return [x, y];
  }

  function nearestNode(gx, gy) {
    // Search radius shrinks as we zoom in so the click-target stays consistent on screen.
    const radius = 30 / Math.max(0.2, transform.k);
    return simulation?.find(gx, gy, radius) ?? null;
  }

  let dragMoved = false;

  function onMouseMove(ev) {
    const [gx, gy] = clientToGraph(ev.clientX, ev.clientY);
    const n = nearestNode(gx, gy);
    if (n) {
      hover = n.id;
      // Show name + photo count on node hover. `total` lands on the node from the
      // CoOccurrenceResult (rebuildGraph), which sources it from the cached result.
      const count = n.total ?? 0;
      tooltip = {
        x: ev.clientX, y: ev.clientY,
        text: `${label(n.id)} · ${formatCount(count)} photos`,
      };
      render();
      return;
    }
    hover = null;
    const e = hitTestEdge(edges, gx, gy, 6 / transform.k);
    if (e) {
      const wText = settings.edgeMode === 'jaccard'
        ? `${e.displayWeight.toFixed(2)} jaccard`
        : `${e.count} photos`;
      tooltip = {
        x: ev.clientX, y: ev.clientY,
        text: `${label(e.source.id)} ↔ ${label(e.target.id)} · ${wText}`,
      };
    } else {
      tooltip = null;
    }
    render();
  }

  function onClick(ev) {
    if (dragMoved) { dragMoved = false; return; }
    const [gx, gy] = clientToGraph(ev.clientX, ev.clientY);
    const n = nearestNode(gx, gy);
    // Alt+click on a node deep-links into the Immich /people/{id} page.
    // We use Alt (rather than Shift) to leave Shift free for browser- and
    // user-defined multi-select gestures.
    //
    // Subtle: even when triggered programmatically, browsers honor the live
    // keyboard state for target=_blank navigations — Shift maps to *new window*,
    // Ctrl/Cmd to *new tab*, Alt to *download*. We force a clean new-tab open
    // by dispatching an explicit MouseEvent with every modifier flag set to
    // false, which Chromium / Firefox prefer over the live state. The anchor
    // is inserted into the DOM because some browsers ignore clicks on
    // detached elements.
    if (n && ev.altKey && graphState.immichBaseUrl) {
      const url = `${graphState.immichBaseUrl}/people/${encodeURIComponent(n.id)}`;
      const a = document.createElement('a');
      a.href = url;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      a.style.cssText = 'position:fixed;left:-9999px;top:-9999px;';
      document.body.appendChild(a);
      a.dispatchEvent(new MouseEvent('click', {
        view: window,
        bubbles: true,
        cancelable: true,
        ctrlKey: false,
        shiftKey: false,
        metaKey: false,
        altKey: false,
      }));
      a.remove();
      return;
    }
    locked = n ? n.id : null;
    render();
  }

  function onDblClick(ev) {
    const [gx, gy] = clientToGraph(ev.clientX, ev.clientY);
    const n = nearestNode(gx, gy);
    if (n) { n.fx = null; n.fy = null; simulation?.alpha(0.3).restart(); }
  }

  $effect(() => {
    if (!canvasEl) return;
    onResize();
    const ro = new ResizeObserver(onResize);
    ro.observe(canvasEl);
    window.addEventListener('resize', onResize);
    return () => {
      ro.disconnect();
      window.removeEventListener('resize', onResize);
    };
  });

  // Tracks which result the simulation was last *fully rebuilt* for. Stays in sync with
  // graphState.result through rebuildGraph(). The rebind effect uses it to skip the
  // duplicate fire that would otherwise race a fresh rebuildGraph() on first result load.
  let resultBuiltFor = null;

  $effect(() => {
    // React to graphState.result changes — full rebuild
    const r = graphState.result;
    rebuildGraph();
    resultBuiltFor = r;
  });

  $effect(() => {
    // Reweight on edge mode or min weight changes — preserves positions.
    // Skip when the result itself just changed (rebuildGraph handles weights).
    settings.edgeMode; settings.minEdgeWeight;
    if (graphState.result && simulation && resultBuiltFor === graphState.result) {
      rebindEdges();
    }
  });

  $effect(() => {
    // Toggling "Show unnamed" changes the displayable node set — rebuild the
    // simulation from scratch. Layout re-explodes but the toggle is rare.
    // Track `showUnnamed` only; untrack the condition so this effect doesn't
    // re-fire (and double-rebuild) when graphState.result changes — that path
    // is already handled by the rebuildGraph effect above.
    settings.showUnnamed;
    untrack(() => {
      if (graphState.result && simulation && resultBuiltFor === graphState.result) {
        rebuildGraph();
      }
    });
  });

  // Wire d3-zoom + d3-drag
  $effect(() => {
    if (!canvasEl) return;
    const sel = select(canvasEl);

    const zoomBehavior = zoom()
      .scaleExtent([0.2, 4])
      .on('zoom', (ev) => { transform = { k: ev.transform.k, x: ev.transform.x, y: ev.transform.y }; render(); });

    sel.call(zoomBehavior);

    const dragBehavior = drag()
      // Only claim the gesture when the cursor is over a node. Otherwise let zoom
      // handle the mousedown so empty-space drags pan the canvas.
      .filter((ev) => {
        if (ev.button !== undefined && ev.button !== 0) return false;
        const [gx, gy] = clientToGraph(ev.clientX, ev.clientY);
        return nearestNode(gx, gy) !== null;
      })
      .subject((ev) => {
        const [gx, gy] = clientToGraph(ev.sourceEvent.clientX, ev.sourceEvent.clientY);
        return nearestNode(gx, gy);
      })
      .on('start', (ev) => {
        if (!ev.subject) return;
        dragMoved = false;
        if (!ev.active) simulation.alphaTarget(0.3).restart();
        ev.subject.fx = ev.subject.x;
        ev.subject.fy = ev.subject.y;
      })
      .on('drag', (ev) => {
        if (!ev.subject) return;
        dragMoved = true;
        const [gx, gy] = clientToGraph(ev.sourceEvent.clientX, ev.sourceEvent.clientY);
        ev.subject.fx = gx;
        ev.subject.fy = gy;
      })
      .on('end', (ev) => {
        if (!ev.subject) return;
        if (!ev.active) simulation.alphaTarget(0);
        ev.subject.fx = null;
        ev.subject.fy = null;
      });

    sel.call(dragBehavior);

    return () => {
      sel.on('.zoom', null);
      sel.on('.drag', null);
    };
  });

  onDestroy(() => { simulation?.stop(); });

  // Exposed via `bind:this` for ExportFab → snapshot of the live scene.
  export function getScene() {
    return {
      width, height,
      nodes, edges,
      displayMode, label,
      imageCache,
      colors: {
        edge: cssVar('--edge') || '#0891B2',
        text: cssVar('--foreground') || '#EDEDEF',
        surface: cssVar('--bg-elevated') || '#0a0a0c',
        border: 'rgba(255,255,255,0.12)',
      },
    };
  }
</script>

<canvas
  bind:this={canvasEl}
  onmousemove={onMouseMove}
  onclick={onClick}
  ondblclick={onDblClick}
></canvas>

{#if tooltip}
  <div class="tooltip" style:left="{tooltip.x + 12}px" style:top="{tooltip.y + 12}px">{tooltip.text}</div>
{/if}

<style>
  canvas {
    width: 100%; height: 100%;
    display: block;
    cursor: grab;
  }
  canvas:active { cursor: grabbing; }
  .tooltip {
    position: fixed;
    pointer-events: none;
    padding: 4px 8px;
    background: rgba(10, 10, 12, 0.92);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 12px;
    z-index: 30;
  }
</style>
