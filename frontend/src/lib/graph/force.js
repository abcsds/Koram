import { forceSimulation, forceLink, forceManyBody, forceCenter, forceCollide } from 'd3-force';

export function computeNodeRadius(total) {
  const r = 12 + Math.sqrt(Math.max(0, total)) * 1.5;
  return Math.max(12, Math.min(40, r));
}

const MIN_LINK_STRENGTH = 0.05;

export function weightToStrength(weight, maxWeight) {
  if (maxWeight <= 0) return 0;
  return Math.min(1, Math.max(MIN_LINK_STRENGTH, weight / maxWeight));
}

/**
 * Build (or rebuild) a d3-force simulation for the given graph.
 * `nodes` are mutated in place by d3 (each gets x, y, vx, vy, fx, fy).
 * `edges` get `source`/`target` rebound to node references.
 */
export function buildSimulation(nodes, edges, width, height) {
  const maxWeight = edges.reduce((m, e) => Math.max(m, e.weight), 0);
  edges.forEach(e => { e.weightNorm = weightToStrength(e.weight, maxWeight); });

  return forceSimulation(nodes)
    .force('link', forceLink(edges)
      .id(d => d.id)
      .distance(d => 80 + 200 / (1 + d.weightNorm))
      .strength(d => d.weightNorm))
    .force('charge', forceManyBody().strength(-300).distanceMax(800))
    .force('center', forceCenter(width / 2, height / 2).strength(0.05))
    .force('collide', forceCollide(d => d.radius + 4));
}

/** Recompute weights on edges after switching count↔jaccard. Call before .alpha(0.6).restart(). */
export function reweight(edges) {
  const maxWeight = edges.reduce((m, e) => Math.max(m, e.weight), 0);
  edges.forEach(e => { e.weightNorm = weightToStrength(e.weight, maxWeight); });
}
