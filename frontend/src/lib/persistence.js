import { STORAGE_KEYS } from './constants.js';
import { debounce } from './utils.js';

function safeGet(key) { try { return localStorage.getItem(key); } catch { return null; } }
function safeSet(key, val) { try { localStorage.setItem(key, val); } catch { /* quota */ } }

export function hydrate(settings) {
  const sel = safeGet(STORAGE_KEYS.selected);
  if (sel) try { settings.setSelected(JSON.parse(sel)); } catch {}
  const showUnnamed = safeGet(STORAGE_KEYS.showUnnamed);
  if (showUnnamed != null) settings.showUnnamed = showUnnamed === 'true';
  const dm = safeGet(STORAGE_KEYS.displayMode);
  if (dm === 'thumbnail' || dm === 'name') settings.displayMode = dm;
  const overrides = safeGet(STORAGE_KEYS.perPersonOverrides);
  if (overrides) try { settings.perPersonOverrides = JSON.parse(overrides); } catch {}
  const em = safeGet(STORAGE_KEYS.edgeMode);
  if (em === 'count' || em === 'jaccard') settings.edgeMode = em;
  const minW = safeGet(STORAGE_KEYS.minEdgeWeight);
  if (minW != null) settings.minEdgeWeight = parseFloat(minW) || 1;
  const df = safeGet(STORAGE_KEYS.dateFrom); if (df != null) settings.dateFrom = df;
  const dt = safeGet(STORAGE_KEYS.dateTo);   if (dt != null) settings.dateTo = dt;
  const drawer = safeGet(STORAGE_KEYS.drawerOpen);
  if (drawer != null) settings.drawerOpen = drawer === 'true';
  const search = safeGet(STORAGE_KEYS.search);
  if (search != null) settings.search = search;
}

const persist = debounce((settings) => {
  safeSet(STORAGE_KEYS.selected, JSON.stringify([...settings.selected]));
  safeSet(STORAGE_KEYS.showUnnamed, String(settings.showUnnamed));
  safeSet(STORAGE_KEYS.displayMode, settings.displayMode);
  safeSet(STORAGE_KEYS.perPersonOverrides, JSON.stringify(settings.perPersonOverrides));
  safeSet(STORAGE_KEYS.edgeMode, settings.edgeMode);
  safeSet(STORAGE_KEYS.minEdgeWeight, String(settings.minEdgeWeight));
  safeSet(STORAGE_KEYS.dateFrom, settings.dateFrom);
  safeSet(STORAGE_KEYS.dateTo, settings.dateTo);
  safeSet(STORAGE_KEYS.drawerOpen, String(settings.drawerOpen));
  safeSet(STORAGE_KEYS.search, settings.search);
}, 300);

/** Wire up auto-persistence by tracking each rune-backed field. Call once after hydrate. */
export function trackPersistence(settings) {
  $effect(() => {
    // Read all watched fields so the effect re-runs on any change.
    settings.selected; settings.showUnnamed; settings.displayMode;
    settings.perPersonOverrides; settings.edgeMode; settings.minEdgeWeight;
    settings.dateFrom; settings.dateTo; settings.drawerOpen; settings.search;
    persist(settings);
  });
}
