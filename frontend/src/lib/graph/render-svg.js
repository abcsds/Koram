function escape(s) {
  return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

export function buildSvg({ width, height, nodes, edges, displayMode, label, thumbnailDataUri, colors }) {
  const defs = nodes.map(n => {
    const uri = thumbnailDataUri(n.id);
    if (!uri) return '';
    return `<pattern id="p-${escape(n.id)}" x="0" y="0" width="1" height="1">
      <image href="${uri}" x="0" y="0" width="${n.radius * 2}" height="${n.radius * 2}" preserveAspectRatio="xMidYMid slice" />
    </pattern>`;
  }).join('');

  const lines = edges.map(e =>
    `<line x1="${e.source.x.toFixed(2)}" y1="${e.source.y.toFixed(2)}"
           x2="${e.target.x.toFixed(2)}" y2="${e.target.y.toFixed(2)}"
           stroke="${colors.edge}" stroke-opacity="${(0.4 + 0.6 * e.weightNorm).toFixed(3)}"
           stroke-width="${(1 + 2 * e.weightNorm).toFixed(2)}" />`
  ).join('');

  const nodesSvg = nodes.map(n => {
    const mode = displayMode(n.id);
    if (mode === 'name' || !thumbnailDataUri(n.id)) {
      const text = escape(label(n.id));
      const padX = 8, h = 22;
      const w = Math.max(40, text.length * 7) + padX * 2;
      return `<g transform="translate(${n.x.toFixed(2)},${n.y.toFixed(2)})">
        <rect x="${-w/2}" y="${-h/2}" width="${w}" height="${h}" rx="11" ry="11"
              fill="${colors.surface}" stroke="${colors.border}" />
        <text x="0" y="0" text-anchor="middle" dominant-baseline="middle"
              font-family="Inter, sans-serif" font-size="13" font-weight="500" fill="${colors.text}">${text}</text>
      </g>`;
    }
    const fill = `url(#p-${escape(n.id)})`;
    return `<circle cx="${n.x.toFixed(2)}" cy="${n.y.toFixed(2)}" r="${n.radius}"
                    fill="${fill}" stroke="${colors.border}" stroke-width="1" />`;
  }).join('');

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
    <defs>${defs}</defs>
    <rect width="${width}" height="${height}" fill="${colors.surface}" />
    ${lines}
    ${nodesSvg}
  </svg>`;
}
