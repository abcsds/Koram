export const API = {
  health: '/api/health',
  connection: '/api/connection',
  people: '/api/people',
  personThumb: (id) => `/api/people/${encodeURIComponent(id)}/thumbnail`,
  graphCompute: '/api/graph/compute',
  graphResult: (key) => `/api/graph/result?key=${encodeURIComponent(key)}`,
  graphCancel: '/api/graph/cancel',
  upload: '/api/upload',
  config: '/api/config',
  ws: '/api/ws',
};

export const STORAGE_KEYS = {
  selected: 'koram.selected',
  showUnnamed: 'koram.showUnnamed',
  displayMode: 'koram.displayMode',
  perPersonOverrides: 'koram.perPerson',
  edgeMode: 'koram.edgeMode',
  minEdgeWeight: 'koram.minEdgeWeight',
  dateFrom: 'koram.dateFrom',
  dateTo: 'koram.dateTo',
  drawerOpen: 'koram.drawerOpen',
  search: 'koram.search',
};
