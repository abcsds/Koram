# Koram — Face Co-Occurrence Graph for Immich

**Status:** Design
**Date:** 2026-05-06
**Owner:** beto
**Reference plugin:** [`immich-automated-selfie-timelapse`](../../../immich-automated-selfie-timelapse/) (architectural template)

## Summary

Koram is a self-hosted Immich plugin that turns the face recognition data already in your Immich library into an interactive force-directed graph. Each node is a person; each edge weight is the number of photos that contain both people. Nodes are draggable, the graph re-flows live by weight, and the result can be exported as PNG, CSV, or uploaded back to Immich as an asset in a dedicated album.

The product is a single fullscreen canvas with one piece of chrome: a hamburger menu that opens a settings drawer.

## Goals

1. Show **who appears with whom** in your photo library, in a glanceable, interactive way.
2. Make it trivial to **steer the layout**: drag nodes, watch them re-settle by co-occurrence weight.
3. Let the user **share or archive** the graph: PNG, CSV, or push it back into Immich.
4. Match the existing Immich-plugin pattern (Rust + Axum backend, Svelte 5 frontend, Docker Compose deployment) so it slots into a fleet of similarly-built tools.

## Non-goals

- No face *recognition* or *clustering* — Koram trusts Immich's existing person assignments.
- No editing of names or merging of people — done in Immich itself.
- No graph algorithms beyond co-occurrence and Jaccard similarity (no community detection, no centrality scoring) in v1.
- No multi-user / per-user views — single instance, single API key.
- No mobile-first layout. Desktop / large tablet primary; mobile is best-effort.

## User stories

- *As a photo-library owner,* I want to see which family members are most often photographed together, so I can find connection patterns I didn't notice.
- *As someone curating a memory book,* I want to export a clean PNG of the social graph, so I can include it.
- *As a tinkerer,* I want a CSV of the underlying co-occurrence data, so I can analyse it elsewhere.
- *As an Immich user,* I want the graph saved back to my library, so it lives alongside the photos that produced it.

---

## Architecture

### Stack

- **Backend:** Rust + Axum 0.8, single binary, port 5000
- **Frontend:** Svelte 5 (runes mode) + Vite 6, built static
- **Graph engine:** D3.js `d3-force` + HTML Canvas (live), inline SVG (export only)
- **Container:** Multi-stage Dockerfile (Node frontend build → Rust release build → Ubuntu 24.04 runtime)
- **Persistence:** TOML config in `/app/config/koram.toml`; co-occurrence cache as JSON in `/app/cache/`

This stack is identical to the reference plugin so patterns transfer line-for-line: Immich client, config, app state, WebSocket progress, Docker layout.

### Folder layout

```
koram/
├── Cargo.toml
├── Dockerfile
├── README.md
├── .dockerignore
├── .gitignore
├── docs/
│   └── specs/
│       └── 2026-05-06-koram-face-graph-design.md
├── frontend/
│   ├── package.json
│   ├── svelte.config.js
│   ├── vite.config.js
│   ├── index.html
│   └── src/
│       ├── main.js
│       ├── App.svelte
│       ├── styles/global.css
│       └── lib/
│           ├── components/
│           │   ├── TopBar.svelte
│           │   ├── SettingsDrawer.svelte
│           │   ├── PeopleList.svelte
│           │   ├── PersonRow.svelte
│           │   ├── DateRange.svelte
│           │   ├── DisplayControls.svelte
│           │   ├── ExportFab.svelte
│           │   ├── GraphCanvas.svelte
│           │   └── ConnectionStatus.svelte
│           ├── graph/
│           │   ├── force.js          # d3-force config; weight → strength mapping
│           │   ├── render-canvas.js  # canvas draw loop (live)
│           │   ├── render-svg.js     # offscreen SVG snapshot (export)
│           │   ├── jaccard.js        # client-side normalization
│           │   └── png-export.js     # SVG → 2× rasterized PNG blob
│           ├── api.js
│           ├── store.js              # Svelte 5 runes; settings + graph state
│           ├── persistence.js        # localStorage hydrate/dehydrate
│           ├── constants.js
│           └── utils.js
└── src/
    ├── main.rs
    ├── lib.rs
    ├── error.rs
    ├── config.rs
    ├── immich_api/
    │   └── mod.rs
    ├── cooccurrence/
    │   ├── mod.rs
    │   ├── compute.rs
    │   └── cache.rs
    ├── job/
    │   └── mod.rs
    └── web/
        ├── mod.rs
        ├── state.rs
        └── handlers/
            ├── mod.rs
            ├── health.rs
            ├── people.rs
            ├── graph.rs
            ├── upload.rs
            ├── ws.rs
            └── config.rs
```

### Backend modules

#### `immich_api`

Adapted from the reference plugin's client. Reuses `Person`, `Asset`, `PersonWithFaces`, `SearchParams`, `ImmichClient::new`, `validate_connection`, `get_people`, `get_person_thumbnail`, `search_person_assets` (paginated), and adds:

- `upload_asset(bytes, device_asset_id, file_created_at) -> AssetId` — `POST /assets` (multipart).
- `ensure_album(name) -> AlbumId` — `GET /albums`, find by name, else `POST /albums`.
- `add_assets_to_album(album_id, asset_ids)` — `PUT /albums/{id}/assets`.

Required Immich API key permissions (`README.md`):

- `album.create`, `album.read`, `album.update`
- `asset.read`, `asset.upload`, `asset.view`, `asset.download`
- `person.read`
- `server.about`

#### `cooccurrence::compute`

Per-person sweep strategy (chosen for incremental progress and natural scaling with the user's selection).

```
fn compute(client, selected_person_ids, from?, to?, cancel_token, progress_tx)
  -> CoOccurrenceResult
```

1. Initialize `assets: HashMap<AssetId, HashSet<PersonId>>` and `totals: HashMap<PersonId, u32>`.
2. For each `person_id` in `selected_person_ids` (concurrent fan-out, `buffer_unordered(8)`):
   - Call `search_person_assets(person_id, from, to, None)` (lifted from reference plugin).
   - For each returned asset: `assets.entry(asset.id).or_default().extend(asset.people)`; bump `totals[person_id]`.
   - Emit progress: `(person_id, processed_count, total_count)`.
   - Honor `cancel_token`.
3. After all sweeps complete, build `pairs: HashMap<(PersonId, PersonId), u32>` (sorted tuple key).
4. For each asset's person-set, for each unordered pair within the intersection of `set` and `selected_person_ids`, increment `pairs[(a, b)]`.
5. Return `CoOccurrenceResult { people: [{id, name, total}], pairs: [{a, b, count}], computed_at, from, to }`.

Edge cases:

- A person in the selection with zero photos in range: included in `people` with `total: 0`, contributes no edges.
- An asset with only one selected person in its face set: counted toward that person's `total` only, no edges.
- Unnamed people are addressed by their Immich person ID; the frontend decides how to render them.

#### `cooccurrence::cache`

Disk cache keyed by:

```
key = sha256(
    sorted(selected_person_ids).join(",")
    || "|" || from.unwrap_or("") || "|" || to.unwrap_or("")
)
```

Stored as `/app/cache/<key>.json` (the `CoOccurrenceResult` directly). Returned immediately on subsequent matching requests. `?force=true` deletes before recomputing. Cache files are written atomically (write to `.tmp` → rename) to avoid partial reads.

#### `web::state` (`AppState`)

Mirrors the reference plugin's pattern:

- `Arc<RwLock<Config>>` — shared config
- `Arc<RwLock<JobState>>` — current compute job (idle, running, completed, error)
- `broadcast::Sender<Progress>` — WebSocket fan-out
- `Arc<RwLock<Option<CancellationToken>>>` — cancellation handle

#### `web::handlers` — API surface

| Method | Path | Purpose |
|---|---|---|
| `GET`  | `/api/health` | Liveness check. |
| `GET`  | `/api/connection` | `{ok: bool, immich_version: string?}`. |
| `GET`  | `/api/people` | `[{id, name, photo_count}]`. `photo_count` from Immich's `/people` endpoint if available, else lazy-loaded. |
| `GET`  | `/api/people/{id}/thumbnail` | Proxied face thumbnail bytes; `Cache-Control: public, max-age=3600`. |
| `POST` | `/api/graph/compute` | Body `{person_ids: string[], from?: string, to?: string, force?: bool}`. Starts compute (or returns cached). Response `{job_id, cached: bool, result?: CoOccurrenceResult}`. |
| `GET`  | `/api/graph/result?key=...` | Returns cached result by key (304 semantics if unchanged). |
| `POST` | `/api/graph/cancel` | Cancels the active job. |
| `GET`  | `/api/ws` | WebSocket. Messages: `{status, processed, total, current_person_id?, current_person_name?, message?}`. |
| `POST` | `/api/upload` | Multipart `image/png`. Backend uploads to Immich, ensures `Koram Graphs` album, links asset, returns `{asset_id, album_id}`. |
| `GET`  | `/api/config` / `PUT /api/config` | Persisted config. |
| `GET`  | `/api/config/defaults` | Defaults endpoint (reset support). |
| Static | `/` | Serves `frontend/dist/` with SPA fallback. |

### Frontend modules

#### `store.js` (Svelte 5 runes)

```js
export const settings = $state({
  selected: new Set(),         // person IDs
  showUnnamed: false,
  displayMode: 'thumbnail',    // 'thumbnail' | 'name'
  perPersonOverrides: {},      // { personId: 'thumbnail' | 'name' }
  edgeMode: 'count',           // 'count' | 'jaccard'
  minEdgeWeight: 1,            // 1..N for count, 0..1 for jaccard
  dateFrom: null,
  dateTo: null,
  search: '',
});

export const graph = $state({
  status: 'idle',              // 'idle' | 'computing' | 'ready' | 'error'
  result: null,                // CoOccurrenceResult
  progress: { processed: 0, total: 0, currentPersonName: null },
  error: null,
});
```

Persistence (`persistence.js`) hydrates `settings` from `localStorage` on startup and writes (debounced 300ms) on any change.

#### `graph/force.js` — D3 force config

```js
forceSimulation(nodes)
  .force('link',   forceLink(edges).id(d => d.id)
                       .distance(d => 80 + 200 / (1 + d.weightNorm))
                       .strength(d => Math.min(1, d.weightNorm)))
  .force('charge', forceManyBody().strength(-300).distanceMax(800))
  .force('center', forceCenter(width/2, height/2).strength(0.05))
  .force('collide', forceCollide(d => d.radius + 4));
```

`weightNorm` = `weight / maxWeight` regardless of mode. Mode swap recomputes `weight` and `weightNorm` then `simulation.alpha(0.6).restart()`. Pinned nodes keep their `(fx, fy)`.

#### `graph/render-canvas.js`

Per simulation tick: clear → draw edges (low alpha, width = `1 + 2 × weightNorm`, color `--edge` with opacity `weightNorm`) → draw nodes (circle + cached `HTMLImageElement` clipped to circle, OR text label in `name-only` mode) → optional captions when zoomed past 1.5×.

DPR-aware sizing: `canvas.width = clientWidth * devicePixelRatio`. Node hit-testing via `simulation.find(x, y, radius)`. Edge hit-testing: project the cursor onto each edge segment, return the closest within 6px (linear scan; fine at <5,000 edges).

Node radius is `12 + sqrt(person.total) * 1.5`, clamped to `[12, 40]`. The `radius` value flows through `forceCollide` so layout respects it.

Hover state: highlighted node + adjacent edges in full opacity; non-adjacent dimmed to 0.2. Click locks the highlight; click on background clears.

Drag: standard `d3-drag` pattern; `dragstarted` sets `(fx, fy)`, `dragged` updates, `dragended` *keeps* them set (pinning). Double-click clears `(fx, fy)`.

#### `graph/render-svg.js` + `png-export.js`

On export click:

1. Snapshot current `(x, y)` of every node, current `weight` of every edge, current display mode.
2. Build an `<svg>` element off-DOM at viewport size:
   - Edges as `<line>` with `stroke-opacity` per weight.
   - Nodes as `<circle>` with `<defs><pattern>` referencing the cached thumbnail as a data URI.
   - Labels as `<text>` in Inter, rendered through an embedded `@font-face` data URI in a `<style>` block to keep the export self-contained.
3. Serialize SVG → load into `Image()` → draw to an offscreen `<canvas>` at `2× viewport size` → `canvas.toBlob('image/png')`.

This same blob is used for download (`URL.createObjectURL` + anchor click) and for upload (POST multipart to `/api/upload`).

#### `graph/jaccard.js`

```
jaccard(pair_count, total_a, total_b) =
    pair_count / (total_a + total_b - pair_count)
```

Computed in the browser when `edgeMode === 'jaccard'`. The backend always sends raw counts and per-person totals; mode switching never requires a refetch.

#### CSV export

Format:

```
person_a_id,person_a_name,person_b_id,person_b_name,photo_count,jaccard
```

Includes only currently-visible pairs (post `minEdgeWeight` filter, post-selection). Sorted by `photo_count` descending. Names are CSV-escaped (quote and double internal quotes).

---

## UX

### Visual style

**Modern Dark Cinema** + **Photo Editor & Filters** palette. Premium, dark, photo-library native. Zero "cyberpunk gimmick" — no scanlines, no neon glow on text, no monospace headlines.

#### Tokens

```css
:root {
  --bg-deep: #020203;
  --bg-base: #050506;
  --bg-elevated: #0a0a0c;
  --surface: rgba(255, 255, 255, 0.04);
  --border: rgba(255, 255, 255, 0.08);
  --foreground: #EDEDEF;
  --foreground-muted: #8A8F98;
  --accent: #7C3AED;
  --accent-glow: rgba(124, 58, 237, 0.20);
  --edge: #0891B2;
  --destructive: #DC2626;
  --radius: 16px;
  --radius-control: 8px;
  --easing: cubic-bezier(0.16, 1, 0.3, 1);
}
```

#### Typography

- **Inter** (300/400/500/600) — all UI text, including names on nodes
- **JetBrains Mono** (400/500) — numerics: counts, weights, dates

Loaded via `@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&family=JetBrains+Mono:wght@400;500&display=swap')`.

#### Effects

- Drawer: `backdrop-filter: blur(20px)`; hairline left border in `--border`
- Hover on node: `--accent` ring + soft halo (radial-gradient stamp on canvas)
- Drag: cursor `grabbing`, node scales to 1.06; spring back via `--easing`
- Reduced-motion: drag still works, transitions snap to end state

#### Avoided

Pure `#000000`, neon glow on text, scanlines, glitch animations, animated background blobs.

### Layout

Single fullscreen canvas with floating chrome:

- **Top bar** (44px, glass surface): hamburger (left), live `nodes / edges` counter in JetBrains Mono (center), connection dot + Immich version on hover (right). Auto-hides after 3s of cursor inactivity over the canvas; returns on mousemove or any keypress. Always visible for keyboard users (focus-within keeps it visible).
- **Settings drawer** (right side, 380px wide): slides in over the canvas, backdrop blur. Doesn't block canvas in the unblurred area. `Esc` closes; click-outside on canvas closes; sticky footer with **Refresh from Immich** primary button.
- **Export FAB** (bottom-right): single collapsed glass button (48×48). Click expands to reveal three vertically-stacked icon buttons:
  - `⤓ PNG` — download current view
  - `⤓ CSV` — download data
  - `⇪ Immich` — upload to "Koram Graphs" album
  Brief inline confirmation (checkmark, fades in 1.5s) on success; toast for errors. Click outside or `Esc` collapses.

### Canvas interactions

- **Drag node** → pin it (releases force on that node); double-click → unpin
- **Scroll / pinch** → zoom (cursor-anchored, 0.2× to 4×)
- **Drag empty space** → pan
- **Hover edge** → tooltip `Person A ↔ Person B · 42 photos` (or `· 0.32 jaccard` in jaccard mode)
- **Hover node** → highlight node + adjacent edges; non-adjacent dim to 0.2 opacity
- **Click node** → persist highlight; click background → clear

### Settings drawer contents

```
┌─ Settings ──────────────────────── ✕ ─┐
│ Date range                            │
│   From [2020-01-01]  To [2026-05-06]  │
│   ☐ All time                          │
│                                       │
│ Display                               │
│   Node style                          │
│     ●  Face thumbnail                 │
│     ○  Name only (named faces only)   │
│                                       │
│   Edge weight                         │
│     ●  Photo count        (raw)       │
│     ○  Jaccard similarity (norm.)     │
│                                       │
│   Min edge weight   [  2  ]           │
│                                       │
│ People                          47    │
│   [🔍 Search…              ]          │
│   ☑ Show unnamed faces                │
│   ─────────────────────────────       │
│   ☑ Select all   ☐ None               │
│                                       │
│   ☑ [👤] Mary Smith         142       │
│        └─ ●  thumbnail  ○ name        │
│   ☑ [👤] Diego Ortiz        118       │
│        └─ ○ thumbnail  ●  name        │
│   ☑ [👤] (unnamed #4f2)      34       │
│   …                                   │
├───────────────────────────────────────┤
│  [ ↻  Refresh from Immich           ] │ ← sticky footer
└───────────────────────────────────────┘
```

#### Date range
Two `<input type="date">` fields. "All time" disables both. Changes mark cache stale (amber dot on Refresh).

#### Display
- `Node style`: radio. **Name only** is disabled with tooltip when any selected person is unnamed (would render blank).
- `Edge weight`: radio between Photo count and Jaccard. Switching is instant (client-side).
- `Min edge weight`: **mode-aware control**.
  - In `count` mode: integer spinner, label "Min photos", min 1.
  - In `jaccard` mode: 0.00–1.00 slider with two-decimal readout, label "Min similarity".

#### People
- Search filters by name (case-insensitive); doesn't change selection.
- `Show unnamed faces`: when off, unnamed people are hidden from the list. When on, they appear with their face ID stub.
- `Select all / None` operates on the currently-visible (post-search, post-unnamed-filter) list.
- Each row: checkbox · 32px circular thumbnail · name · photo count (Mono, muted, right-aligned).
- Per-row sub-toggle (visible only when row is checked AND person has a name): thumbnail / name display override. Defaults to global Display setting; overridable per-person.

**Resolution rule for what a node renders as**:

```
displayFor(person) =
    if perPersonOverrides[person.id] exists  → that value
    else if person has no name              → 'thumbnail'  (always; name-only would be blank)
    else                                    → settings.displayMode
```
- Rows virtualized; libraries with 1000+ named people stay smooth.

#### Refresh footer
- Idle: "↻ Refresh from Immich"
- Stale (settings changed): "↻ Refresh (settings changed)" + amber dot
- Computing: spinner + "Fetching · Mary Smith · 12 / 47"; button switches to **Cancel**

### Persistence

**localStorage** (debounced 300ms): selection, display mode, per-person overrides, date range, edge mode, min edge weight, search text, drawer open/closed.

**Backend disk cache** (`/app/cache/<key>.json`): the actual `CoOccurrenceResult` keyed by `sha256(person_ids|from|to)`.

### Empty / error states

- **First load** → centered card "Connecting to Immich…" → "Found 47 named people · select faces to graph"
- **Computing** → progress bar in the top bar replaces the counter: `Fetching photos · Mary Smith · 12/47`
- **No edges yet** → friendly overlay "Pick at least 2 people from the menu" with hamburger pulse hint
- **Empty after compute** (no pairs ≥ min weight) → "No co-occurrences. Lower min weight or pick more people." Drawer auto-opens.
- **Immich unreachable / 401** → red dot + banner "Can't reach Immich. Check API key in `/app/config/koram.toml`." No partial graph drawn.
- **Compute failed mid-stream** → progress bar in `--destructive`; partial result offered: "Got 12 of 47 — show partial graph?"
- **Upload to Immich fails** → error toast + retry; PNG downloaded locally as fallback.
- **localStorage quota exceeded** → silently drop persistence; selection works in-memory; logged to console.
- **Stale cache references a person who's been merged or deleted in Immich** → on result load, the frontend filters out person IDs no longer in `/api/people`; if any are missing, the Refresh button gets the amber "stale" dot with tooltip "Some people were removed from Immich".

---

## Performance budget

- **Frontend**: smooth at 300 nodes / 5,000 edges on a mid-range laptop. Canvas (not SVG) is the load-bearing decision here.
- **Backend compute**: dominated by Immich's `/search/metadata` latency (~100ms × N selected people for typical libraries). Sweep is concurrent up to 8 in parallel via `futures::stream::iter().buffer_unordered(8)`.
- **Image cache**: browser pre-fetches face thumbnails on selection. ~10KB each. 100 selected people = ~1MB held in `HTMLImageElement` cache.

---

## Testing strategy

### Rust unit tests (`#[cfg(test)]`)

- `cooccurrence::compute::tests` — given a synthetic asset/person fixture, produce expected pair counts. Covers:
  - Empty selection → empty result
  - Single person → no pairs, totals correct
  - Two people, no shared photos → totals only, no pairs
  - Three people with overlapping presence → all three pair counts correct
  - Person appears multiple times in same asset (face data quirk) → counted once
- `cache::key::tests` — same inputs produce same key; person-ID order doesn't change key (sorted internally); different date range changes key.
- `immich_api::tests` — base URL sanitization (lifted from reference plugin), pagination loop bounded.

### Integration tests (`#[ignore]` by default; `cargo test -- --ignored`)

- Hit the public Immich demo server (same as reference plugin).
- Fetch first 5 people, run `compute`, assert nonzero `people` and that returned `pair.count` ≤ `min(totals[a], totals[b])`.

### Frontend unit tests (Vitest)

- `jaccard.js` — math: zero pair, identical sets, asymmetric totals.
- `force.js` config — `strength()` returns within `[0, 1]` across edge cases (zero weight, max weight, empty graph).
- `png-export.js` — produces non-empty Blob from a small fixture (3 nodes, 2 edges).

No Svelte component tests in v1; rely on manual browser verification per the project's UI rule.

### Manual verification before shipping

Run against own Immich instance:

1. Golden path: pick 5 people → compute → drag → export PNG → upload to Immich → verify in `Koram Graphs` album.
2. Switch edge mode (count ↔ jaccard) — graph re-flows without refetch.
3. Refresh after changing date range — old cache discarded.
4. Cancel mid-compute — partial result offered, then refresh succeeds.
5. Edge cases: 0 selected, 1 selected, 100+ selected, all unnamed.
6. Reduced-motion: verify no jarring animations.

---

## Out of scope (v1)

- Community detection, clustering, centrality algorithms
- Time-lapse animation of the graph as photos are added
- Multi-tenant / per-user views
- Editing names / merging people inside Koram
- Mobile-optimized layout
- Authentication / user login (single-instance, single API key)
- Custom album name (always "Koram Graphs")
- Album selection per upload

---

## Open questions

None — all clarifying questions resolved during brainstorming on 2026-05-06.

---

## Appendix: Required Immich API permissions

Grant the API key these permissions when creating it in Immich:

- `album.create`
- `album.read`
- `album.update`
- `asset.read`
- `asset.upload`
- `asset.view`
- `asset.download`
- `person.read`
- `server.about`

## Appendix: Docker Compose example

```yaml
services:
  koram:
    image: koram:latest
    container_name: koram
    user: 1000:1000
    ports:
      - "5001:5000"
    environment:
      - IMMICH_API_KEY=your-api-key
      - IMMICH_BASE_URL=http://your-immich-host:2283
    volumes:
      - ./config:/app/config
      - ./cache:/app/cache
    restart: unless-stopped
```

(Port 5001 to avoid clashing with the selfie-timelapse plugin on 5000.)
