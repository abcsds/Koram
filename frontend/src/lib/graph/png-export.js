import { buildSvg } from './render-svg.js';

export async function imageToDataUri(img) {
  if (!img || !img.complete) return null;
  const c = document.createElement('canvas');
  c.width = img.naturalWidth || 64;
  c.height = img.naturalHeight || 64;
  const ctx = c.getContext('2d');
  try { ctx.drawImage(img, 0, 0); } catch { return null; }
  return c.toDataURL('image/jpeg', 0.85);
}

export async function exportPng({ width, height, nodes, edges, displayMode, label, imageCache, colors, scale = 2 }) {
  const dataUriCache = new Map();
  for (const n of nodes) {
    dataUriCache.set(n.id, await imageToDataUri(imageCache.get(n.id)));
  }
  const thumbnailDataUri = (id) => dataUriCache.get(id) ?? null;

  const svg = buildSvg({ width, height, nodes, edges, displayMode, label, thumbnailDataUri, colors });
  const blob = new Blob([svg], { type: 'image/svg+xml' });
  const url = URL.createObjectURL(blob);

  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const c = document.createElement('canvas');
      c.width = width * scale;
      c.height = height * scale;
      const ctx = c.getContext('2d');
      ctx.scale(scale, scale);
      ctx.drawImage(img, 0, 0);
      c.toBlob((png) => {
        URL.revokeObjectURL(url);
        png ? resolve(png) : reject(new Error('canvas.toBlob returned null'));
      }, 'image/png');
    };
    img.onerror = (e) => { URL.revokeObjectURL(url); reject(e); };
    img.src = url;
  });
}
