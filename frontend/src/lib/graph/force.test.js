import { describe, it, expect } from 'vitest';
import { computeNodeRadius, weightToStrength } from './force.js';

describe('computeNodeRadius', () => {
  it('clamps to 12 minimum', () => {
    expect(computeNodeRadius(0)).toBe(12);
  });
  it('clamps to 40 maximum', () => {
    expect(computeNodeRadius(100000)).toBe(40);
  });
  it('scales with sqrt of total', () => {
    const r1 = computeNodeRadius(4);
    const r2 = computeNodeRadius(16);
    expect(r2 - 12).toBeGreaterThan(r1 - 12);
  });
});

describe('weightToStrength', () => {
  it('returns 0 for empty graph (maxWeight=0)', () => {
    expect(weightToStrength(0, 0)).toBe(0);
  });
  it('returns 0..1 within range', () => {
    expect(weightToStrength(5, 10)).toBe(0.5);
    expect(weightToStrength(10, 10)).toBe(1);
  });
  it('caps at 1 even if weight exceeds max', () => {
    expect(weightToStrength(20, 10)).toBe(1);
  });
  it('floors strength at 0.05 for non-empty graphs so tiny edges still pull', () => {
    // A graph with one heavy edge and many tiny ones — tiny edges shouldn't go to ~0.
    expect(weightToStrength(1, 100)).toBeGreaterThanOrEqual(0.05);
  });
});
