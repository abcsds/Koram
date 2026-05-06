import { describe, it, expect } from 'vitest';
import { jaccard } from './jaccard.js';

describe('jaccard', () => {
  it('returns 0 for zero pair count', () => {
    expect(jaccard(0, 5, 5)).toBe(0);
  });
  it('returns 1 for identical sets', () => {
    expect(jaccard(5, 5, 5)).toBe(1);
  });
  it('handles asymmetric totals', () => {
    expect(jaccard(2, 4, 6)).toBeCloseTo(0.25, 5);
  });
  it('handles zero totals safely', () => {
    expect(jaccard(0, 0, 0)).toBe(0);
  });
});
