const NODE_RING_PX = 1;
const HOVER_HALO_PX = 16;

/**
 * Per-tick canvas draw.
 * scene = { nodes, edges, transform: {k, x, y}, hover: nodeId|null, locked: nodeId|null,
 *           imageCache: Map<id, HTMLImageElement>, displayMode: (id) => 'thumbnail'|'name',
 *           label: (id) => string }
 */
export function draw(ctx, width, height, scene) {
  const dpr = window.devicePixelRatio || 1;
  ctx.save();
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, width, height);
  ctx.translate(scene.transform.x, scene.transform.y);
  ctx.scale(scene.transform.k, scene.transform.k);

  drawEdges(ctx, scene);
  drawNodes(ctx, scene);

  ctx.restore();
}

function isHighlighted(scene, nodeId) {
  const focus = scene.locked ?? scene.hover;
  if (!focus) return true;
  if (focus === nodeId) return true;
  return scene.adjacency.get(focus)?.has(nodeId) ?? false;
}

function edgeIsHighlighted(scene, edge) {
  const focus = scene.locked ?? scene.hover;
  if (!focus) return true;
  return edge.source.id === focus || edge.target.id === focus;
}

function drawEdges(ctx, scene) {
  for (const e of scene.edges) {
    const w = e.weightNorm;
    const focused = edgeIsHighlighted(scene, e);
    ctx.globalAlpha = focused ? 0.4 + 0.6 * w : 0.1;
    ctx.lineWidth = 1 + 2 * w;
    ctx.strokeStyle = scene.edgeColor;
    ctx.beginPath();
    ctx.moveTo(e.source.x, e.source.y);
    ctx.lineTo(e.target.x, e.target.y);
    ctx.stroke();
  }
  ctx.globalAlpha = 1;
}

function drawNodes(ctx, scene) {
  for (const n of scene.nodes) {
    const focused = isHighlighted(scene, n.id);
    ctx.globalAlpha = focused ? 1 : 0.2;

    const mode = scene.displayMode(n.id);
    const r = n.radius;

    if (mode === 'name') {
      // Pill with text
      const label = scene.label(n.id);
      ctx.font = '500 13px Inter, sans-serif';
      const m = ctx.measureText(label);
      const padX = 8;
      const w = m.width + padX * 2;
      const h = 22;
      ctx.fillStyle = '#0a0a0c';
      ctx.strokeStyle = focused && (scene.locked === n.id || scene.hover === n.id) ? scene.accentColor : 'rgba(255,255,255,0.12)';
      ctx.lineWidth = 1;
      roundRect(ctx, n.x - w / 2, n.y - h / 2, w, h, 11);
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = '#EDEDEF';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(label, n.x, n.y);
    } else {
      // Circular thumbnail
      const img = scene.imageCache.get(n.id);
      ctx.save();
      ctx.beginPath();
      ctx.arc(n.x, n.y, r, 0, Math.PI * 2);
      ctx.closePath();
      ctx.clip();
      if (img && img.complete && img.naturalWidth > 0) {
        ctx.drawImage(img, n.x - r, n.y - r, r * 2, r * 2);
      } else {
        ctx.fillStyle = '#171939';
        ctx.fill();
      }
      ctx.restore();

      // Halo for hovered/locked
      if (scene.locked === n.id || scene.hover === n.id) {
        const grad = ctx.createRadialGradient(n.x, n.y, r, n.x, n.y, r + HOVER_HALO_PX);
        grad.addColorStop(0, scene.accentGlow);
        grad.addColorStop(1, 'rgba(0,0,0,0)');
        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.arc(n.x, n.y, r + HOVER_HALO_PX, 0, Math.PI * 2);
        ctx.fill();
      }

      // Ring
      ctx.lineWidth = NODE_RING_PX;
      ctx.strokeStyle = (scene.locked === n.id || scene.hover === n.id) ? scene.accentColor : 'rgba(255,255,255,0.12)';
      ctx.beginPath();
      ctx.arc(n.x, n.y, r, 0, Math.PI * 2);
      ctx.stroke();
    }
  }
  ctx.globalAlpha = 1;
}

function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y,     x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x,     y + h, r);
  ctx.arcTo(x,     y + h, x,     y,     r);
  ctx.arcTo(x,     y,     x + w, y,     r);
  ctx.closePath();
}

/**
 * Edge hit-test: return the edge whose perpendicular distance to (px, py) is < threshold,
 * with the smallest distance. Returns null otherwise.
 * (px, py) are in graph (untransformed) coordinates.
 */
export function hitTestEdge(edges, px, py, threshold = 6) {
  let best = null;
  let bestD = threshold;
  for (const e of edges) {
    const x1 = e.source.x, y1 = e.source.y, x2 = e.target.x, y2 = e.target.y;
    const dx = x2 - x1, dy = y2 - y1;
    const len2 = dx * dx + dy * dy;
    if (len2 === 0) continue;
    const t = Math.max(0, Math.min(1, ((px - x1) * dx + (py - y1) * dy) / len2));
    const cx = x1 + t * dx, cy = y1 + t * dy;
    const d = Math.hypot(px - cx, py - cy);
    if (d < bestD) { bestD = d; best = e; }
  }
  return best;
}
