import { describe, it, expect, vi } from 'vitest';

vi.stubGlobal('fetch', () => Promise.resolve({
  ok: true,
  blob: () => Promise.resolve(new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' })),
}));

import { buildSvg } from './render-svg.js';

describe('buildSvg', () => {
  it('produces an svg string with given viewport size', () => {
    const svg = buildSvg({
      width: 200, height: 100,
      nodes: [
        { id: 'a', x: 50, y: 50, radius: 12, name: 'A' },
        { id: 'b', x: 150, y: 50, radius: 12, name: 'B' },
      ],
      edges: [{ source: { id: 'a', x: 50, y: 50 }, target: { id: 'b', x: 150, y: 50 }, weightNorm: 1 }],
      displayMode: () => 'name',
      label: (id) => id.toUpperCase(),
      thumbnailDataUri: () => null,
      colors: { edge: '#0891B2', text: '#EDEDEF', surface: '#0a0a0c', border: 'rgba(255,255,255,0.12)' },
    });
    expect(svg).toContain('<svg');
    expect(svg).toContain('width="200"');
    expect(svg).toContain('height="100"');
    expect(svg).toContain('A');
    expect(svg).toContain('B');
  });
});
