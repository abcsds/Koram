// Svelte 5 runes-based stores. Imported by components which then use $state.snapshot etc.
// We keep mutable state in a class instance to satisfy the runes-only-in-components rule.

class Settings {
  selected = $state(new Set());
  showUnnamed = $state(false);
  displayMode = $state('thumbnail');         // 'thumbnail' | 'name'
  perPersonOverrides = $state({});            // { [personId]: 'thumbnail' | 'name' }
  edgeMode = $state('count');                 // 'count' | 'jaccard'
  minEdgeWeight = $state(1);                  // number; 1..N for count, 0..1 for jaccard
  dateFrom = $state('');
  dateTo = $state('');
  search = $state('');
  drawerOpen = $state(true);

  toggleSelected(id) {
    if (this.selected.has(id)) this.selected.delete(id);
    else this.selected.add(id);
    this.selected = new Set(this.selected); // trigger reactivity
  }
  setSelected(ids) { this.selected = new Set(ids); }
  setOverride(id, mode) {
    if (mode === null) {
      const { [id]: _, ...rest } = this.perPersonOverrides;
      this.perPersonOverrides = rest;
    } else {
      this.perPersonOverrides = { ...this.perPersonOverrides, [id]: mode };
    }
  }
}

class GraphState {
  status = $state('idle');                    // 'idle' | 'computing' | 'ready' | 'error'
  result = $state(null);                      // CoOccurrenceResult
  error = $state(null);
  progress = $state({ processed: 0, total: 0, currentPersonName: null });
}

export const settings = new Settings();
export const graphState = new GraphState();
