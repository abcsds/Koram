import { API } from './constants.js';

async function jsonOrThrow(res) {
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    throw new Error(`${res.status} ${res.statusText}: ${text}`);
  }
  return res.json();
}

export async function getConnection() {
  return jsonOrThrow(await fetch(API.connection));
}

export async function getPeople() {
  return jsonOrThrow(await fetch(API.people));
}

export async function computeGraph(body) {
  return jsonOrThrow(await fetch(API.graphCompute, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  }));
}

export async function cancelGraph() {
  return jsonOrThrow(await fetch(API.graphCancel, { method: 'POST' }));
}

export async function uploadToImmich(blob, deviceAssetId) {
  const fd = new FormData();
  fd.append('image', blob, `${deviceAssetId}.png`);
  fd.append('deviceAssetId', deviceAssetId);
  return jsonOrThrow(await fetch(API.upload, { method: 'POST', body: fd }));
}

export function openProgressSocket(onMessage) {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const ws = new WebSocket(`${proto}://${location.host}${API.ws}`);
  ws.onmessage = (e) => {
    try { onMessage(JSON.parse(e.data)); } catch {}
  };
  return ws;
}
