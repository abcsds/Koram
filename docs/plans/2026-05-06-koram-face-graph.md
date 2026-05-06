# Koram — Face Co-Occurrence Graph Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build koram, a self-hosted Immich plugin that renders an interactive force-directed graph of face co-occurrence (which people appear in photos with which other people), with PNG/CSV/Immich-album exports.

**Architecture:** Rust + Axum backend (single binary) calls Immich's `/people` and `/search/metadata` endpoints, computes a co-occurrence matrix per-person, caches it on disk. Svelte 5 + D3.js frontend renders the graph live to HTML Canvas, exports via offscreen SVG → PNG. Multi-stage Dockerfile pattern matching the existing `immich-automated-selfie-timelapse` plugin.

**Tech Stack:** Rust 2021 / tokio / axum 0.8 / reqwest 0.12 / serde / sha2; Svelte 5 (runes mode) / Vite 6 / d3 (`d3-force`, `d3-drag`, `d3-zoom`, `d3-selection`).

**Reference plugin:** `/home/beto/code/immich-automated-selfie-timelapse/` — the `immich_api/mod.rs`, `web/state.rs`, `web/handlers/*.rs`, and `Dockerfile` are the patterns to follow.

**Spec:** `koram/docs/specs/2026-05-06-koram-face-graph-design.md`

---

## File Structure

```
koram/
├── Cargo.toml                           # Task 1
├── Dockerfile                           # Task 1
├── README.md                            # Task 35
├── .dockerignore                        # Task 1
├── .gitignore                           # Task 1
├── frontend/
│   ├── package.json                     # Task 2
│   ├── vite.config.js                   # Task 2
│   ├── svelte.config.js                 # Task 2
│   ├── index.html                       # Task 2
│   └── src/
│       ├── main.js                      # Task 2
│       ├── App.svelte                   # Task 34
│       ├── styles/global.css            # Task 18
│       └── lib/
│           ├── api.js                   # Task 19
│           ├── constants.js             # Task 19
│           ├── store.js                 # Task 20
│           ├── persistence.js           # Task 20
│           ├── utils.js                 # Task 19
│           ├── components/
│           │   ├── ConnectionStatus.svelte  # Task 21
│           │   ├── TopBar.svelte            # Task 22
│           │   ├── PersonRow.svelte         # Task 23
│           │   ├── PeopleList.svelte        # Task 24
│           │   ├── DateRange.svelte         # Task 25
│           │   ├── DisplayControls.svelte   # Task 26
│           │   ├── SettingsDrawer.svelte    # Task 27
│           │   ├── GraphCanvas.svelte       # Task 31
│           │   └── ExportFab.svelte         # Task 33
│           └── graph/
│               ├── jaccard.js               # Task 28
│               ├── force.js                 # Task 29
│               ├── render-canvas.js         # Task 30
│               ├── render-svg.js            # Task 32
│               └── png-export.js            # Task 32
└── src/
    ├── main.rs                          # Task 17
    ├── lib.rs                           # Task 3
    ├── error.rs                         # Task 3
    ├── config.rs                        # Task 4
    ├── immich_api/mod.rs                # Tasks 5–7
    ├── cooccurrence/
    │   ├── mod.rs                       # Task 9
    │   ├── compute.rs                   # Task 9
    │   └── cache.rs                     # Task 8
    ├── job/mod.rs                       # Task 10
    └── web/
        ├── mod.rs                       # Task 17
        ├── state.rs                     # Task 11
        └── handlers/
            ├── mod.rs                   # Task 17 (router)
            ├── health.rs                # Task 12
            ├── people.rs                # Task 13
            ├── graph.rs                 # Task 14
            ├── ws.rs                    # Task 15
            ├── upload.rs                # Task 16
            └── config.rs                # Task 16
```

---

## Conventions

**TDD throughout.** Every task with logic writes the failing test first, runs it to confirm failure, implements, runs to confirm pass, commits.

**One commit per task** with a `feat:` / `chore:` / `test:` prefix. Always end with `git add` / `git commit`. Commits are not optional checkpoints — they're the unit of progress.

**Frontend tests** use Vitest. Run with `cd frontend && npx vitest run <path>`.

**Backend tests** run with `cargo test --lib <name>`. Demo-server integration tests are gated with `#[ignore]` and run with `cargo test -- --ignored`.

**Working directory** for all `cd` and tool commands is `/home/beto/code/koram` unless otherwise stated.

---

## Task 1: Project bootstrap — Cargo, Dockerfile, gitignore

**Files:**
- Create: `koram/Cargo.toml`
- Create: `koram/Dockerfile`
- Create: `koram/.dockerignore`
- Create: `koram/.gitignore`

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "koram"
version = "0.1.0"
edition = "2021"
description = "Interactive face co-occurrence graph for Immich"
license = "MIT"

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = { version = "0.8", features = ["ws", "multipart"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["fs", "cors", "set-header"] }
reqwest = { version = "0.12", features = ["json", "stream", "multipart"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
directories = "5"
bytes = "1"
tokio-util = "0.7"
futures-util = "0.3"
async-trait = "0.1"
dotenvy = "0.15"
urlencoding = "2.1.3"
sha2 = "0.10"
hex = "0.4"

[dev-dependencies]
tokio-test = "0.4"

[[bin]]
name = "koram"
path = "src/main.rs"
```

- [ ] **Step 2: Create `Dockerfile`** (mirrors the reference plugin's pattern, minus the ML downloads since koram has no ONNX/dlib dependencies)

```dockerfile
# Stage 1: Build frontend
FROM node:22-alpine AS frontend-build
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust binary
FROM ubuntu:24.04 AS rust-build
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates \
    cmake g++ make pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Cache dependency build
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && \
    echo 'fn main() { println!("dummy"); }' > src/main.rs && \
    echo '' > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Build real binary
COPY src/ src/
RUN touch src/main.rs src/lib.rs && cargo build --release

# Stage 3: Runtime image
FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-build /app/target/release/koram ./
COPY --from=frontend-build /app/frontend/dist/ ./frontend/dist/

RUN mkdir -p config cache

EXPOSE 5000
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fs http://localhost:5000/api/health || exit 1
CMD ["./koram"]
```

- [ ] **Step 3: Create `.dockerignore`**

```
target/
node_modules/
frontend/node_modules/
frontend/dist/
.git/
config/
cache/
*.md
docs/
```

- [ ] **Step 4: Create `.gitignore`**

```
target/
node_modules/
frontend/dist/
config/
cache/
.env
*.swp
.DS_Store
```

- [ ] **Step 5: Initialize git and verify Cargo parses**

```bash
cd /home/beto/code/koram
git init
cargo check 2>&1 | head -3 || true   # Will fail (no src yet); we just want Cargo.toml syntax confirmed by next task
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Dockerfile .dockerignore .gitignore
git commit -m "chore: bootstrap koram project"
```

---

## Task 2: Frontend scaffolding — Vite, Svelte 5

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/vite.config.js`
- Create: `frontend/svelte.config.js`
- Create: `frontend/index.html`
- Create: `frontend/src/main.js`
- Create: `frontend/src/App.svelte` (placeholder)

- [ ] **Step 1: Create `frontend/package.json`**

```json
{
  "name": "koram-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "test": "vitest run"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^5.0.0",
    "svelte": "^5.0.0",
    "vite": "^6.0.0",
    "vitest": "^2.0.0",
    "jsdom": "^25.0.0"
  },
  "dependencies": {
    "d3-drag": "^3.0.0",
    "d3-force": "^3.0.0",
    "d3-selection": "^3.0.0",
    "d3-zoom": "^3.0.0"
  }
}
```

- [ ] **Step 2: Create `frontend/vite.config.js`**

```js
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:5000',
    },
  },
  test: {
    environment: 'jsdom',
  },
});
```

- [ ] **Step 3: Create `frontend/svelte.config.js`**

```js
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
export default { preprocess: vitePreprocess() };
```

- [ ] **Step 4: Create `frontend/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Koram</title>
    <link rel="icon" href="data:," />
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.js"></script>
  </body>
</html>
```

- [ ] **Step 5: Create `frontend/src/main.js`**

```js
import { mount } from 'svelte';
import App from './App.svelte';
import './styles/global.css';

const app = mount(App, { target: document.getElementById('app') });
export default app;
```

- [ ] **Step 6: Create placeholder `frontend/src/App.svelte`**

```svelte
<main>
  <h1>Koram</h1>
  <p>Loading…</p>
</main>
```

- [ ] **Step 7: Create placeholder `frontend/src/styles/global.css`**

```css
body { margin: 0; background: #050506; color: #EDEDEF; font-family: system-ui; }
```

- [ ] **Step 8: Install and verify build**

```bash
cd /home/beto/code/koram/frontend
npm install
npm run build
```

Expected: `dist/index.html` and `dist/assets/` exist. No errors.

- [ ] **Step 9: Commit**

```bash
cd /home/beto/code/koram
git add frontend/
git commit -m "chore: bootstrap frontend (Svelte 5 + Vite)"
```

---

## Task 3: Rust skeleton — `lib.rs`, `error.rs`, empty `main.rs`

**Files:**
- Create: `src/lib.rs`
- Create: `src/error.rs`
- Create: `src/main.rs`

- [ ] **Step 1: Create `src/error.rs`**

```rust
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Immich API error: {0}")]
    ImmichApi(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Operation cancelled")]
    Cancelled,
}

pub const PERMISSION_HINT: &str =
    "Ensure the volume mounts (/app/config, /app/cache) are writable by the container user (uid 1000 by default).";
```

- [ ] **Step 2: Create `src/lib.rs`**

```rust
pub mod config;
pub mod cooccurrence;
pub mod error;
pub mod immich_api;
pub mod job;
pub mod web;
```

- [ ] **Step 3: Create stub `src/main.rs`** (will be fleshed out in Task 17)

```rust
fn main() {
    println!("koram bootstrap — full main wired up in Task 17");
}
```

- [ ] **Step 4: Stub the modules so `lib.rs` compiles**

```bash
cd /home/beto/code/koram
mkdir -p src/immich_api src/cooccurrence src/job src/web/handlers
```

Create each as one-line stubs:

```bash
echo "pub mod placeholder {}" > src/config.rs
printf "pub mod compute;\npub mod cache;\n" > src/cooccurrence/mod.rs
echo "pub fn _stub() {}" > src/cooccurrence/compute.rs
echo "pub fn _stub() {}" > src/cooccurrence/cache.rs
echo "pub fn _stub() {}" > src/immich_api/mod.rs
echo "pub fn _stub() {}" > src/job/mod.rs
printf "pub mod handlers;\npub mod state;\n" > src/web/mod.rs
echo "pub fn _stub() {}" > src/web/state.rs
echo "pub fn _stub() {}" > src/web/handlers/mod.rs
```

- [ ] **Step 5: Build**

```bash
cargo build 2>&1 | tail -5
```

Expected: clean compile (warnings about unused `_stub` are fine).

- [ ] **Step 6: Commit**

```bash
git add src/ Cargo.lock
git commit -m "chore: add Rust skeleton with module stubs"
```

---

## Task 4: `config.rs` — TOML config + env overrides

**Files:**
- Replace: `src/config.rs`
- Test: `src/config.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing test** in `src/config.rs`

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CONFIG_PATH: &str = "config/koram.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub api_key: String,
    pub base_url: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 { 30 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api: ApiConfig,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    #[serde(default = "default_album_name")]
    pub album_name: String,
}

fn default_cache_dir() -> PathBuf { PathBuf::from("cache") }
fn default_album_name() -> String { "Koram Graphs".into() }

impl Config {
    pub fn from_env() -> Self {
        Self {
            api: ApiConfig {
                api_key: std::env::var("IMMICH_API_KEY").unwrap_or_default(),
                base_url: std::env::var("IMMICH_BASE_URL").unwrap_or_default(),
                timeout_secs: 30,
            },
            cache_dir: default_cache_dir(),
            album_name: default_album_name(),
        }
    }

    pub fn from_file(path: &std::path::Path) -> crate::error::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        toml::from_str(&s).map_err(|e| crate::error::Error::Config(e.to_string()))
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> crate::error::Result<()> {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let s = toml::to_string_pretty(self).map_err(|e| crate::error::Error::Config(e.to_string()))?;
        std::fs::write(path, s)?;
        Ok(())
    }

    pub fn with_env(mut self) -> Self {
        if let Ok(v) = std::env::var("IMMICH_API_KEY") { if !v.is_empty() { self.api.api_key = v; } }
        if let Ok(v) = std::env::var("IMMICH_BASE_URL") { if !v.is_empty() { self.api.base_url = v; } }
        self
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        if self.api.api_key.is_empty() {
            return Err(crate::error::Error::Config("IMMICH_API_KEY not set".into()));
        }
        if self.api.base_url.is_empty() {
            return Err(crate::error::Error::Config("IMMICH_BASE_URL not set".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn validate_rejects_empty_key() {
        let cfg = Config {
            api: ApiConfig { api_key: "".into(), base_url: "http://x".into(), timeout_secs: 30 },
            cache_dir: "cache".into(),
            album_name: "K".into(),
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn roundtrip_toml() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("c.toml");
        let cfg = Config {
            api: ApiConfig { api_key: "abc".into(), base_url: "http://x".into(), timeout_secs: 30 },
            cache_dir: "cache".into(),
            album_name: "Koram Graphs".into(),
        };
        cfg.save_to_file(&p).unwrap();
        let loaded = Config::from_file(&p).unwrap();
        assert_eq!(loaded.api.api_key, "abc");
        assert_eq!(loaded.album_name, "Koram Graphs");
    }
}
```

- [ ] **Step 2: Add `tempfile` to dev-dependencies**

In `Cargo.toml`:
```toml
[dev-dependencies]
tokio-test = "0.4"
tempfile = "3"
```

- [ ] **Step 3: Run tests**

```bash
cargo test --lib config
```

Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/config.rs Cargo.toml Cargo.lock
git commit -m "feat: config module with TOML + env overrides"
```

---

## Task 5: Immich client — base + URL sanitization + connection

**Files:**
- Replace: `src/immich_api/mod.rs`

This is largely a port from `/home/beto/code/immich-automated-selfie-timelapse/src/immich_api/mod.rs`. Only the parts koram needs are copied; new methods (`upload_asset`, `ensure_album`, `add_assets_to_album`) come in Task 7.

- [ ] **Step 1: Write `src/immich_api/mod.rs`** with types + client + sanitization + connection check

```rust
use crate::config::ApiConfig;
use crate::error::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use urlencoding::encode as urlencode;

#[derive(Clone)]
pub struct ImmichClient {
    client: Client,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo { pub version: String }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub people: Option<Vec<PersonRef>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonRef {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse { assets: SearchAssets }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchAssets {
    items: Vec<Asset>,
    next_page: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    person_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    taken_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    taken_before: Option<String>,
    #[serde(rename = "type")]
    asset_type: String,
    page: u32,
    size: u32,
    with_people: bool,
}

impl ImmichClient {
    pub fn new(config: &ApiConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;
        Ok(Self {
            client,
            base_url: Self::sanitize_base_url(&config.base_url),
            api_key: config.api_key.clone(),
        })
    }

    fn sanitize_base_url(url: &str) -> String {
        let trimmed = url.trim_end_matches('/');
        if trimmed.ends_with("/api") { trimmed.into() } else { format!("{}/api", trimmed) }
    }

    async fn check(resp: reqwest::Response, op: &str) -> Result<reqwest::Response> {
        let status = resp.status();
        if status.is_success() { return Ok(resp); }
        let body = resp.text().await.unwrap_or_default();
        Err(match status {
            reqwest::StatusCode::UNAUTHORIZED => Error::ImmichApi(format!("{op}: invalid or expired API key (401)")),
            reqwest::StatusCode::FORBIDDEN    => Error::ImmichApi(format!("{op}: missing permission (403). {body}")),
            _ => Error::ImmichApi(format!("{op}: HTTP {status}. {body}")),
        })
    }

    pub async fn validate_connection(&self) -> Result<ServerInfo> {
        let url = format!("{}/server/about", self.base_url);
        let resp = self.client.get(&url).header("x-api-key", &self.api_key).send().await?;
        let resp = Self::check(resp, "Validate connection").await?;
        Ok(resp.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_base_url() {
        assert_eq!(ImmichClient::sanitize_base_url("http://x:2283/api"), "http://x:2283/api");
        assert_eq!(ImmichClient::sanitize_base_url("http://x:2283/api/"), "http://x:2283/api");
        assert_eq!(ImmichClient::sanitize_base_url("http://x:2283"), "http://x:2283/api");
        assert_eq!(ImmichClient::sanitize_base_url("http://x:2283/"), "http://x:2283/api");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test --lib immich_api
```

Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add src/immich_api/mod.rs
git commit -m "feat(immich): client base + URL sanitization + validate_connection"
```

---

## Task 6: Immich client — `get_people`, `get_person_thumbnail`, `search_person_assets`

**Files:**
- Modify: `src/immich_api/mod.rs`

- [ ] **Step 1: Append methods to `impl ImmichClient`** (insert before the closing `}` of the impl block)

```rust
    pub async fn get_people(&self) -> Result<Vec<Person>> {
        let url = format!("{}/people", self.base_url);
        let resp = self.client.get(&url).header("x-api-key", &self.api_key).send().await?;
        let resp = Self::check(resp, "Get people").await?;
        #[derive(Deserialize)]
        struct R { people: Vec<Person> }
        let r: R = resp.json().await?;
        Ok(r.people)
    }

    pub async fn get_person_thumbnail(&self, person_id: &str) -> Result<(bytes::Bytes, String)> {
        let url = format!("{}/people/{}/thumbnail", self.base_url, urlencode(person_id));
        let resp = self.client.get(&url).header("x-api-key", &self.api_key).send().await?;
        let resp = Self::check(resp, "Get person thumbnail").await?;
        let ct = resp.headers().get("content-type")
            .and_then(|v| v.to_str().ok()).unwrap_or("image/jpeg").to_string();
        let bytes = resp.bytes().await?;
        Ok((bytes, ct))
    }

    /// Paginated fetch of every asset that contains `person_id`, optionally filtered by date range.
    /// Returns assets with `with_people=true` so the caller can read every face per asset.
    pub async fn search_person_assets(
        &self,
        person_id: &str,
        taken_after: Option<&str>,
        taken_before: Option<&str>,
    ) -> Result<Vec<Asset>> {
        let mut all = Vec::new();
        let mut page = 1u32;
        loop {
            let params = SearchParams {
                person_ids: Some(vec![person_id.to_string()]),
                taken_after: taken_after.map(String::from),
                taken_before: taken_before.map(String::from),
                asset_type: "IMAGE".into(),
                page,
                size: 100,
                with_people: true,
            };
            let url = format!("{}/search/metadata", self.base_url);
            let resp = self.client.post(&url)
                .header("x-api-key", &self.api_key)
                .json(&params)
                .send().await?;
            let resp = Self::check(resp, "Search assets").await?;
            let r: SearchResponse = resp.json().await?;
            all.extend(r.assets.items);
            if r.assets.next_page.is_none() { break; }
            page += 1;
            if page > 1000 { tracing::warn!("Pagination cap hit"); break; }
        }
        Ok(all)
    }
```

- [ ] **Step 2: Add a smoke test against the public Immich demo (`#[ignore]`)**

Append inside the existing `#[cfg(test)] mod tests`:

```rust
    fn demo_client() -> ImmichClient {
        ImmichClient::new(&ApiConfig {
            api_key: "1bpgd3LpG30Zr3IEPNV3sWhIEqMuUGmzK3jWNh59JU".into(),
            base_url: "https://demo.immich.app/api".into(),
            timeout_secs: 30,
        }).unwrap()
    }

    #[tokio::test]
    #[ignore]
    async fn demo_get_people() {
        let people = demo_client().get_people().await.unwrap();
        assert!(!people.is_empty(), "demo server returned no people");
    }
```

- [ ] **Step 3: Run unit tests (skipping `#[ignore]`)**

```bash
cargo test --lib immich_api
```

Expected: still 1 test passes (the ignored one is skipped).

- [ ] **Step 4: Optionally verify against demo (manual; skip in CI)**

```bash
cargo test --lib immich_api -- --ignored
```

Expected: passes (requires internet).

- [ ] **Step 5: Commit**

```bash
git add src/immich_api/mod.rs
git commit -m "feat(immich): get_people, get_person_thumbnail, search_person_assets"
```

---

## Task 7: Immich client — upload + album management

**Files:**
- Modify: `src/immich_api/mod.rs`

- [ ] **Step 1: Add types and methods** — append to `src/immich_api/mod.rs`

After the existing types, before the `impl ImmichClient`, add:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub album_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResponse {
    pub id: String,
}
```

Inside `impl ImmichClient`, append:

```rust
    pub async fn get_albums(&self) -> Result<Vec<Album>> {
        let url = format!("{}/albums", self.base_url);
        let resp = self.client.get(&url).header("x-api-key", &self.api_key).send().await?;
        let resp = Self::check(resp, "Get albums").await?;
        Ok(resp.json().await?)
    }

    pub async fn create_album(&self, name: &str) -> Result<Album> {
        let url = format!("{}/albums", self.base_url);
        let body = serde_json::json!({ "albumName": name });
        let resp = self.client.post(&url)
            .header("x-api-key", &self.api_key)
            .json(&body).send().await?;
        let resp = Self::check(resp, "Create album").await?;
        Ok(resp.json().await?)
    }

    pub async fn ensure_album(&self, name: &str) -> Result<Album> {
        for a in self.get_albums().await? {
            if a.album_name == name { return Ok(a); }
        }
        self.create_album(name).await
    }

    pub async fn add_assets_to_album(&self, album_id: &str, asset_ids: &[String]) -> Result<()> {
        let url = format!("{}/albums/{}/assets", self.base_url, urlencode(album_id));
        let body = serde_json::json!({ "ids": asset_ids });
        let resp = self.client.put(&url)
            .header("x-api-key", &self.api_key)
            .json(&body).send().await?;
        Self::check(resp, "Add assets to album").await?;
        Ok(())
    }

    /// Upload a PNG asset. Returns the new asset's Immich ID.
    pub async fn upload_asset(
        &self,
        png_bytes: bytes::Bytes,
        device_asset_id: &str,
        file_created_at: &str,
    ) -> Result<UploadResponse> {
        let url = format!("{}/assets", self.base_url);

        let part = reqwest::multipart::Part::bytes(png_bytes.to_vec())
            .file_name(format!("{}.png", device_asset_id))
            .mime_str("image/png")
            .map_err(|e| Error::ImmichApi(format!("Mime: {e}")))?;

        let form = reqwest::multipart::Form::new()
            .text("deviceAssetId", device_asset_id.to_string())
            .text("deviceId", "koram".to_string())
            .text("fileCreatedAt", file_created_at.to_string())
            .text("fileModifiedAt", file_created_at.to_string())
            .part("assetData", part);

        let resp = self.client.post(&url)
            .header("x-api-key", &self.api_key)
            .multipart(form).send().await?;
        let resp = Self::check(resp, "Upload asset").await?;
        Ok(resp.json().await?)
    }
```

- [ ] **Step 2: Compile**

```bash
cargo build 2>&1 | tail -3
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/immich_api/mod.rs
git commit -m "feat(immich): upload_asset, get_albums, create_album, ensure_album, add_assets_to_album"
```

---

## Task 8: Cache — key generation + atomic read/write

**Files:**
- Replace: `src/cooccurrence/cache.rs`

- [ ] **Step 1: Write the failing test inside `src/cooccurrence/cache.rs`**

```rust
use crate::error::{Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn cache_key(person_ids: &[String], from: Option<&str>, to: Option<&str>) -> String {
    let mut sorted = person_ids.to_vec();
    sorted.sort();
    let mut hasher = Sha256::new();
    hasher.update(sorted.join(",").as_bytes());
    hasher.update(b"|");
    hasher.update(from.unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(to.unwrap_or("").as_bytes());
    hex::encode(hasher.finalize())
}

pub fn cache_path(cache_dir: &Path, key: &str) -> PathBuf {
    cache_dir.join(format!("{}.json", key))
}

pub fn read<T: DeserializeOwned>(cache_dir: &Path, key: &str) -> Result<Option<T>> {
    let p = cache_path(cache_dir, key);
    if !p.exists() { return Ok(None); }
    let s = std::fs::read_to_string(&p)?;
    Ok(Some(serde_json::from_str(&s)?))
}

pub fn write<T: Serialize>(cache_dir: &Path, key: &str, value: &T) -> Result<()> {
    std::fs::create_dir_all(cache_dir)?;
    let p = cache_path(cache_dir, key);
    let tmp = p.with_extension("json.tmp");
    let s = serde_json::to_string(value)?;
    std::fs::write(&tmp, s)?;
    std::fs::rename(&tmp, &p).map_err(Error::from)?;
    Ok(())
}

pub fn delete(cache_dir: &Path, key: &str) -> Result<()> {
    let p = cache_path(cache_dir, key);
    if p.exists() { std::fs::remove_file(p)?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_order_independent() {
        let a = cache_key(&["1".into(), "2".into(), "3".into()], None, None);
        let b = cache_key(&["3".into(), "1".into(), "2".into()], None, None);
        assert_eq!(a, b);
    }

    #[test]
    fn key_changes_with_dates() {
        let ids = vec!["a".into()];
        let a = cache_key(&ids, None, None);
        let b = cache_key(&ids, Some("2020-01-01"), None);
        let c = cache_key(&ids, None, Some("2024-12-31"));
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);
    }

    #[test]
    fn roundtrip_write_read() {
        let dir = tempfile::tempdir().unwrap();
        let key = "test";
        let val = vec![("x".to_string(), 1u32), ("y".into(), 2)];
        write(dir.path(), key, &val).unwrap();
        let loaded: Option<Vec<(String, u32)>> = read(dir.path(), key).unwrap();
        assert_eq!(loaded.unwrap(), val);
    }

    #[test]
    fn read_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let loaded: Option<u32> = read(dir.path(), "nope").unwrap();
        assert_eq!(loaded, None);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test --lib cooccurrence::cache
```

Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/cooccurrence/cache.rs
git commit -m "feat(cooccurrence): disk cache with sorted-key hashing and atomic writes"
```

---

## Task 9: Cooccurrence — compute algorithm with synthetic fixtures

**Files:**
- Replace: `src/cooccurrence/compute.rs`
- Replace: `src/cooccurrence/mod.rs`

- [ ] **Step 1: Define types in `src/cooccurrence/mod.rs`**

```rust
pub mod cache;
pub mod compute;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonNode {
    pub id: String,
    pub name: Option<String>,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairCount {
    pub a: String,
    pub b: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoOccurrenceResult {
    pub people: Vec<PersonNode>,
    pub pairs: Vec<PairCount>,
    pub computed_at: String,
    pub from: Option<String>,
    pub to: Option<String>,
}
```

- [ ] **Step 2: Write the failing test in `src/cooccurrence/compute.rs`**

This task has a **pure algorithmic core** that is testable without the network. The async wrapper that calls Immich is added in Step 3.

```rust
use crate::cooccurrence::{CoOccurrenceResult, PairCount, PersonNode};
use std::collections::{HashMap, HashSet};

/// Pure core: given the per-person sweep results, build the co-occurrence result.
///
/// `selected` — the set of person IDs the user picked.
/// `assets` — for each asset ID, the set of person IDs Immich attributes to it (across the union of all sweeps).
/// `totals` — for each person in `selected`, the count of assets they appeared in.
/// `people_meta` — id → optional name, for the selected people.
pub fn build_result(
    selected: &HashSet<String>,
    assets: &HashMap<String, HashSet<String>>,
    totals: &HashMap<String, u32>,
    people_meta: &HashMap<String, Option<String>>,
    computed_at: String,
    from: Option<String>,
    to: Option<String>,
) -> CoOccurrenceResult {
    let mut pairs: HashMap<(String, String), u32> = HashMap::new();
    for (_asset_id, people_in_asset) in assets {
        let intersection: Vec<&String> = people_in_asset.iter().filter(|p| selected.contains(*p)).collect();
        for i in 0..intersection.len() {
            for j in (i + 1)..intersection.len() {
                let (a, b) = if intersection[i] < intersection[j] {
                    (intersection[i].clone(), intersection[j].clone())
                } else {
                    (intersection[j].clone(), intersection[i].clone())
                };
                *pairs.entry((a, b)).or_insert(0) += 1;
            }
        }
    }

    let mut people: Vec<PersonNode> = selected.iter().map(|id| PersonNode {
        id: id.clone(),
        name: people_meta.get(id).cloned().flatten(),
        total: *totals.get(id).unwrap_or(&0),
    }).collect();
    people.sort_by(|a, b| a.id.cmp(&b.id));

    let mut pair_vec: Vec<PairCount> = pairs.into_iter()
        .map(|((a, b), count)| PairCount { a, b, count })
        .collect();
    pair_vec.sort_by(|x, y| y.count.cmp(&x.count).then_with(|| x.a.cmp(&y.a)));

    CoOccurrenceResult { people, pairs: pair_vec, computed_at, from, to }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    fn s(v: &[&str]) -> HashSet<String> { v.iter().map(|x| x.to_string()).collect() }
    fn m(v: &[(&str, &[&str])]) -> HashMap<String, HashSet<String>> {
        v.iter().map(|(k, vs)| (k.to_string(), s(vs))).collect()
    }
    fn t(v: &[(&str, u32)]) -> HashMap<String, u32> {
        v.iter().map(|(k, c)| (k.to_string(), *c)).collect()
    }
    fn names(v: &[(&str, Option<&str>)]) -> HashMap<String, Option<String>> {
        v.iter().map(|(k, n)| (k.to_string(), n.map(String::from))).collect()
    }

    #[test]
    fn empty_selection_empty_result() {
        let r = build_result(&s(&[]), &m(&[]), &t(&[]), &names(&[]), "now".into(), None, None);
        assert_eq!(r.people.len(), 0);
        assert_eq!(r.pairs.len(), 0);
    }

    #[test]
    fn single_person_no_pairs() {
        let r = build_result(
            &s(&["A"]),
            &m(&[("img1", &["A"]), ("img2", &["A"])]),
            &t(&[("A", 2)]),
            &names(&[("A", Some("Alice"))]),
            "now".into(), None, None,
        );
        assert_eq!(r.people.len(), 1);
        assert_eq!(r.people[0].total, 2);
        assert_eq!(r.pairs.len(), 0);
    }

    #[test]
    fn three_people_overlapping() {
        // imgs:
        //   1: A,B
        //   2: A,B,C
        //   3: A,C
        //   4: B,C
        // selected: {A,B,C}
        // expected pairs: (A,B):2, (A,C):2, (B,C):2
        let r = build_result(
            &s(&["A", "B", "C"]),
            &m(&[
                ("1", &["A", "B"]),
                ("2", &["A", "B", "C"]),
                ("3", &["A", "C"]),
                ("4", &["B", "C"]),
            ]),
            &t(&[("A", 3), ("B", 3), ("C", 3)]),
            &names(&[("A", None), ("B", None), ("C", None)]),
            "now".into(), None, None,
        );
        assert_eq!(r.pairs.len(), 3);
        let lookup: HashMap<(String, String), u32> =
            r.pairs.iter().map(|p| ((p.a.clone(), p.b.clone()), p.count)).collect();
        assert_eq!(lookup[&("A".into(), "B".into())], 2);
        assert_eq!(lookup[&("A".into(), "C".into())], 2);
        assert_eq!(lookup[&("B".into(), "C".into())], 2);
    }

    #[test]
    fn unselected_people_dont_create_edges() {
        // C is in the asset but not selected → no edges involving C
        let r = build_result(
            &s(&["A", "B"]),
            &m(&[("1", &["A", "B", "C"])]),
            &t(&[("A", 1), ("B", 1)]),
            &names(&[("A", None), ("B", None)]),
            "now".into(), None, None,
        );
        assert_eq!(r.pairs.len(), 1);
        assert_eq!(r.pairs[0].a, "A");
        assert_eq!(r.pairs[0].b, "B");
        assert_eq!(r.pairs[0].count, 1);
    }

    #[test]
    fn pair_key_is_sorted() {
        // Even if Immich returns Z before A in the people list, the pair key should be (A, Z)
        let r = build_result(
            &s(&["Z", "A"]),
            &m(&[("1", &["Z", "A"])]),
            &t(&[("A", 1), ("Z", 1)]),
            &names(&[("A", None), ("Z", None)]),
            "now".into(), None, None,
        );
        assert_eq!(r.pairs[0].a, "A");
        assert_eq!(r.pairs[0].b, "Z");
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test --lib cooccurrence::compute
```

Expected: 5 tests pass.

- [ ] **Step 4: Add the async sweep wrapper** — append to `src/cooccurrence/compute.rs`

The concurrency cap is enforced by `futures::stream::buffer_unordered(8)` — exactly the pattern named in the spec's performance budget. Errors are surfaced as `Result<()>` per sweep and merged at the end so a single Immich failure stops the whole compute cleanly.

```rust
use crate::error::Result;
use crate::immich_api::{ImmichClient, Person};
use crate::job::Progress;
use futures_util::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const SWEEP_CONCURRENCY: usize = 8;

pub struct ComputeArgs {
    pub client: ImmichClient,
    pub selected_person_ids: Vec<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub people_meta: Vec<Person>, // names of all known people for label lookups
    pub cancel: CancellationToken,
    pub progress_tx: tokio::sync::broadcast::Sender<Progress>,
}

pub async fn compute(args: ComputeArgs) -> Result<CoOccurrenceResult> {
    use std::collections::{HashMap, HashSet};

    let total = args.selected_person_ids.len() as u32;
    let processed = Arc::new(RwLock::new(0u32));

    let assets: Arc<RwLock<HashMap<String, HashSet<String>>>> = Arc::default();
    let totals: Arc<RwLock<HashMap<String, u32>>> = Arc::default();

    let names_for_progress: HashMap<String, Option<String>> =
        args.people_meta.iter().map(|p| (p.id.clone(), p.name.clone())).collect();

    // Drain the per-person sweep with a strict in-flight cap of 8.
    let results: Vec<Result<()>> = stream::iter(args.selected_person_ids.iter().cloned())
        .map(|id| {
            let client = args.client.clone();
            let from = args.from.clone();
            let to = args.to.clone();
            let assets = assets.clone();
            let totals = totals.clone();
            let processed = processed.clone();
            let cancel = args.cancel.clone();
            let progress_tx = args.progress_tx.clone();
            let names = names_for_progress.clone();
            async move {
                if cancel.is_cancelled() { return Err(crate::error::Error::Cancelled); }
                let fetched = client.search_person_assets(&id, from.as_deref(), to.as_deref()).await?;
                if cancel.is_cancelled() { return Err(crate::error::Error::Cancelled); }

                {
                    let mut a = assets.write().await;
                    let mut t = totals.write().await;
                    t.insert(id.clone(), fetched.len() as u32);
                    for asset in &fetched {
                        let entry = a.entry(asset.id.clone()).or_default();
                        if let Some(people) = &asset.people {
                            for p in people { entry.insert(p.id.clone()); }
                        }
                        entry.insert(id.clone());
                    }
                }

                let mut p = processed.write().await;
                *p += 1;
                let _ = progress_tx.send(Progress {
                    status: "running".into(),
                    processed: *p,
                    total,
                    current_person_id: Some(id.clone()),
                    current_person_name: names.get(&id).cloned().flatten(),
                    message: None,
                });
                Ok(())
            }
        })
        .buffer_unordered(SWEEP_CONCURRENCY)
        .collect()
        .await;

    if args.cancel.is_cancelled() {
        return Err(crate::error::Error::Cancelled);
    }
    // Surface the first error. Sibling sweeps in flight at the same time may still be
    // running; their per-iteration cancel checks let them bail at the next safe point.
    if let Some(first_err) = results.into_iter().find_map(|r| r.err()) {
        args.cancel.cancel(); // signal the rest to abort early
        return Err(first_err);
    }

    let selected: HashSet<String> = args.selected_person_ids.iter().cloned().collect();
    let people_meta: HashMap<String, Option<String>> =
        args.people_meta.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
    let assets_owned = assets.read().await.clone();
    let totals_owned = totals.read().await.clone();

    Ok(build_result(
        &selected, &assets_owned, &totals_owned, &people_meta,
        chrono::Utc::now().to_rfc3339(),
        args.from, args.to,
    ))
}
```

- [ ] **Step 5: Recompile (the wrapper depends on `crate::job::Progress` from Task 10)**

The compile will fail at this point because `crate::job::Progress` doesn't exist yet. That's expected — it'll come together in Task 10.

- [ ] **Step 6: Commit the algorithmic core only**

```bash
git add src/cooccurrence/mod.rs src/cooccurrence/compute.rs
git commit -m "feat(cooccurrence): pair-counting core + async per-person sweep wrapper"
```

---

## Task 10: Job state — Progress type + cancellation

**Files:**
- Replace: `src/job/mod.rs`

- [ ] **Step 1: Write `src/job/mod.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub status: String,             // "idle" | "running" | "completed" | "cancelled" | "error"
    pub processed: u32,
    pub total: u32,
    pub current_person_id: Option<String>,
    pub current_person_name: Option<String>,
    pub message: Option<String>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            status: "idle".into(),
            processed: 0,
            total: 0,
            current_person_id: None,
            current_person_name: None,
            message: None,
        }
    }
}
```

- [ ] **Step 2: Verify the cooccurrence module now compiles**

```bash
cargo build 2>&1 | tail -3
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/job/mod.rs
git commit -m "feat(job): Progress type for streaming compute updates"
```

---

## Task 11: `web::state::AppState`

**Files:**
- Replace: `src/web/state.rs`

- [ ] **Step 1: Write `src/web/state.rs`**

```rust
use crate::config::Config;
use crate::cooccurrence::CoOccurrenceResult;
use crate::job::Progress;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub progress: Arc<RwLock<Progress>>,
    pub progress_tx: broadcast::Sender<Progress>,
    pub cancel_token: Arc<RwLock<Option<CancellationToken>>>,
    pub last_result: Arc<RwLock<Option<CoOccurrenceResult>>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            config: Arc::new(RwLock::new(config)),
            progress: Arc::new(RwLock::new(Progress::default())),
            progress_tx: tx,
            cancel_token: Arc::new(RwLock::new(None)),
            last_result: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn create_cancel_token(&self) -> CancellationToken {
        let token = CancellationToken::new();
        *self.cancel_token.write().await = Some(token.clone());
        token
    }

    pub async fn request_cancel(&self) -> bool {
        if let Some(t) = self.cancel_token.read().await.as_ref() {
            t.cancel();
            true
        } else {
            false
        }
    }
}
```

- [ ] **Step 2: Compile**

```bash
cargo build 2>&1 | tail -3
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/web/state.rs
git commit -m "feat(web): AppState with progress, cancellation, last-result"
```

---

## Task 12: Health + connection handlers

**Files:**
- Create: `src/web/handlers/health.rs`

- [ ] **Step 1: Write `src/web/handlers/health.rs`**

```rust
use crate::immich_api::ImmichClient;
use crate::web::state::AppState;
use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse { pub ok: bool }

pub async fn health_check() -> Json<HealthResponse> { Json(HealthResponse { ok: true }) }

#[derive(Serialize)]
pub struct ConnectionStatus {
    pub ok: bool,
    pub immich_version: Option<String>,
    pub error: Option<String>,
}

pub async fn check_connection(State(state): State<AppState>) -> Json<ConnectionStatus> {
    let cfg = state.config.read().await;
    let client = match ImmichClient::new(&cfg.api) {
        Ok(c) => c,
        Err(e) => return Json(ConnectionStatus { ok: false, immich_version: None, error: Some(e.to_string()) }),
    };
    drop(cfg);
    match client.validate_connection().await {
        Ok(info) => Json(ConnectionStatus { ok: true, immich_version: Some(info.version), error: None }),
        Err(e)   => Json(ConnectionStatus { ok: false, immich_version: None, error: Some(e.to_string()) }),
    }
}

// `StatusCode` is referenced for future use; a `_` import keeps the analyzer quiet without unused-warnings.
#[allow(dead_code)]
fn _doc_status() -> StatusCode { StatusCode::OK }
```

- [ ] **Step 2: Commit**

```bash
git add src/web/handlers/health.rs
git commit -m "feat(web): /api/health and /api/connection handlers"
```

---

## Task 13: People handler — list + thumbnail proxy

**Files:**
- Create: `src/web/handlers/people.rs`

- [ ] **Step 1: Write `src/web/handlers/people.rs`**

```rust
use crate::immich_api::ImmichClient;
use crate::web::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Json, Response},
};
use serde::Serialize;

#[derive(Serialize)]
pub struct PersonInfo {
    pub id: String,
    pub name: Option<String>,
}

pub async fn get_people(State(state): State<AppState>)
    -> Result<Json<Vec<PersonInfo>>, (StatusCode, String)>
{
    let cfg = state.config.read().await;
    let client = ImmichClient::new(&cfg.api)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    drop(cfg);
    let people = client.get_people().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(people.into_iter().map(|p| PersonInfo { id: p.id, name: p.name }).collect()))
}

pub async fn get_person_thumbnail(
    State(state): State<AppState>,
    Path(person_id): Path<String>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let cfg = state.config.read().await;
    let client = ImmichClient::new(&cfg.api)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    drop(cfg);
    let (bytes, ct) = client.get_person_thumbnail(&person_id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, ct)
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
```

- [ ] **Step 2: Commit**

```bash
git add src/web/handlers/people.rs
git commit -m "feat(web): /api/people and /api/people/{id}/thumbnail"
```

---

## Task 14: Graph handler — compute (backgrounded), result, cancel

**Files:**
- Create: `src/web/handlers/graph.rs`

The compute is spawned on a background task so the HTTP response returns immediately. Clients listen on `/api/ws` for progress and then `GET /api/graph/result?key=...` once the WS broadcasts a terminal status. This is what makes `/api/ws` and `/api/graph/cancel` actually meaningful.

- [ ] **Step 1: Write `src/web/handlers/graph.rs`**

```rust
use crate::cooccurrence::{cache, compute, CoOccurrenceResult};
use crate::immich_api::ImmichClient;
use crate::job::Progress;
use crate::web::state::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ComputeRequest {
    pub person_ids: Vec<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Serialize)]
pub struct ComputeResponse {
    pub cached: bool,
    pub result: Option<CoOccurrenceResult>,
    pub key: String,
}

pub async fn compute_graph(
    State(state): State<AppState>,
    Json(req): Json<ComputeRequest>,
) -> Result<Json<ComputeResponse>, (StatusCode, String)> {
    let cfg = state.config.read().await;
    let cache_dir = cfg.cache_dir.clone();
    let client = ImmichClient::new(&cfg.api)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    drop(cfg);

    let key = cache::cache_key(&req.person_ids, req.from.as_deref(), req.to.as_deref());

    // Cache hit short-circuits the job entirely.
    if !req.force {
        if let Some(cached) = cache::read::<CoOccurrenceResult>(&cache_dir, &key)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        {
            *state.last_result.write().await = Some(cached.clone());
            return Ok(Json(ComputeResponse { cached: true, result: Some(cached), key }));
        }
    } else {
        let _ = cache::delete(&cache_dir, &key);
    }

    let people_meta = client.get_people().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let progress_tx = state.progress_tx.clone();
    let total = req.person_ids.len() as u32;

    // Atomic test-and-set: refuse to start if another job is already running, otherwise
    // claim the slot under the same write guard to prevent two POSTs racing past the check.
    let initial = Progress {
        status: "running".into(),
        processed: 0,
        total,
        current_person_id: None,
        current_person_name: None,
        message: None,
    };
    {
        let mut p = state.progress.write().await;
        if p.status == "running" {
            return Err((StatusCode::CONFLICT, "another compute is already running".into()));
        }
        *p = initial.clone();
    }
    let _ = progress_tx.send(initial);

    // Cancellation token created *after* the slot is claimed so a conflicting POST can't
    // overwrite a live token in the AppState.
    let cancel = state.create_cancel_token().await;

    // Spawn the compute. The HTTP request returns immediately.
    let state_bg = state.clone();
    let key_bg = key.clone();
    let cache_dir_bg = cache_dir.clone();
    let req_for_args = req;
    tokio::spawn(async move {
        let args = compute::ComputeArgs {
            client,
            selected_person_ids: req_for_args.person_ids,
            from: req_for_args.from.clone(),
            to: req_for_args.to.clone(),
            people_meta,
            cancel,
            progress_tx: state_bg.progress_tx.clone(),
        };

        let outcome = compute::compute(args).await;

        let final_progress = match &outcome {
            Ok(result) => {
                if let Err(e) = cache::write(&cache_dir_bg, &key_bg, result) {
                    tracing::warn!("Failed to persist cache: {}", e);
                }
                *state_bg.last_result.write().await = Some(result.clone());
                Progress {
                    status: "completed".into(),
                    processed: total,
                    total,
                    current_person_id: None,
                    current_person_name: None,
                    message: Some(key_bg.clone()),
                }
            }
            Err(crate::error::Error::Cancelled) => Progress {
                status: "cancelled".into(),
                processed: 0, total,
                current_person_id: None, current_person_name: None,
                message: None,
            },
            Err(e) => Progress {
                status: "error".into(),
                processed: 0, total,
                current_person_id: None, current_person_name: None,
                message: Some(e.to_string()),
            },
        };

        *state_bg.progress.write().await = final_progress.clone();
        let _ = state_bg.progress_tx.send(final_progress);
    });

    Ok(Json(ComputeResponse { cached: false, result: None, key }))
}

#[derive(Deserialize)]
pub struct ResultQuery { pub key: String }

pub async fn get_result(
    State(state): State<AppState>,
    Query(q): Query<ResultQuery>,
) -> Result<Json<CoOccurrenceResult>, (StatusCode, String)> {
    let cfg = state.config.read().await;
    let cache_dir = cfg.cache_dir.clone();
    drop(cfg);
    let cached = cache::read::<CoOccurrenceResult>(&cache_dir, &q.key)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "not in cache".into()))?;
    Ok(Json(cached))
}

#[derive(Serialize)]
pub struct CancelResponse { pub cancelled: bool }

pub async fn cancel_graph(State(state): State<AppState>) -> Json<CancelResponse> {
    Json(CancelResponse { cancelled: state.request_cancel().await })
}
```

- [ ] **Step 2: Compile**

```bash
cargo build 2>&1 | tail -3
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/web/handlers/graph.rs
git commit -m "feat(web): backgrounded /api/graph/compute with WS progress + cancel"
```

---

## Task 15: WebSocket progress handler

**Files:**
- Create: `src/web/handlers/ws.rs`

- [ ] **Step 1: Write `src/web/handlers/ws.rs`**

```rust
use crate::web::state::AppState;
use axum::{
    extract::{ws::{Message, WebSocket}, State, WebSocketUpgrade},
    response::IntoResponse,
};

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.progress_tx.subscribe();
    {
        let p = state.progress.read().await;
        if let Ok(s) = serde_json::to_string(&*p) {
            let _ = socket.send(Message::Text(s.into())).await;
        }
    }
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(p) => {
                        if let Ok(s) = serde_json::to_string(&p) {
                            if socket.send(Message::Text(s.into())).await.is_err() { break; }
                        }
                    }
                    Err(_) => break,
                }
            }
            inc = socket.recv() => {
                match inc {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/web/handlers/ws.rs
git commit -m "feat(web): WebSocket /api/ws streams Progress updates"
```

---

## Task 16: Upload + config handlers

**Files:**
- Create: `src/web/handlers/upload.rs`
- Create: `src/web/handlers/config.rs`

- [ ] **Step 1: Write `src/web/handlers/upload.rs`**

```rust
use crate::immich_api::ImmichClient;
use crate::web::state::AppState;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct UploadResponse {
    pub asset_id: String,
    pub album_id: String,
}

pub async fn upload_to_immich(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    let mut png_bytes: Option<bytes::Bytes> = None;
    let mut device_asset_id = String::new();

    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart: {e}")))?
    {
        match field.name().unwrap_or("") {
            "image" => {
                let b = field.bytes().await
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
                png_bytes = Some(b);
            }
            "deviceAssetId" => {
                device_asset_id = field.text().await.unwrap_or_default();
            }
            _ => { let _ = field.bytes().await; }
        }
    }

    let png = png_bytes.ok_or((StatusCode::BAD_REQUEST, "missing image field".into()))?;
    if device_asset_id.is_empty() {
        device_asset_id = format!("koram-{}", uuid::Uuid::new_v4());
    }

    let cfg = state.config.read().await;
    let album_name = cfg.album_name.clone();
    let client = ImmichClient::new(&cfg.api)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    drop(cfg);

    let now = chrono::Utc::now().to_rfc3339();
    let upload = client.upload_asset(png, &device_asset_id, &now).await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let album = client.ensure_album(&album_name).await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    client.add_assets_to_album(&album.id, &[upload.id.clone()]).await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    Ok(Json(UploadResponse { asset_id: upload.id, album_id: album.id }))
}
```

- [ ] **Step 2: Write `src/web/handlers/config.rs`**

```rust
use crate::config::Config;
use crate::web::state::AppState;
use axum::{extract::State, http::StatusCode, response::Json};

pub async fn get_config(State(state): State<AppState>) -> Json<Config> {
    Json(state.config.read().await.clone())
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(new): Json<Config>,
) -> Result<Json<Config>, (StatusCode, String)> {
    *state.config.write().await = new.clone();
    new.save_to_file(std::path::Path::new(crate::config::CONFIG_PATH))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(new))
}

pub async fn get_config_defaults() -> Json<Config> {
    // A blank-env config gives us the default `cache_dir` and `album_name`,
    // with API fields empty so the UI can show defaults next to the user's overrides.
    Json(Config::from_env())
}
```

- [ ] **Step 3: Commit**

```bash
git add src/web/handlers/upload.rs src/web/handlers/config.rs
git commit -m "feat(web): /api/upload (Immich PNG + album) and /api/config GET/PUT"
```

---

## Task 17: Router + `main.rs`

**Files:**
- Replace: `src/web/handlers/mod.rs`
- Replace: `src/web/mod.rs`
- Replace: `src/main.rs`

- [ ] **Step 1: Write `src/web/handlers/mod.rs`**

```rust
mod config;
mod graph;
mod health;
mod people;
mod upload;
mod ws;

use crate::web::state::AppState;
use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use tower_http::services::ServeDir;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health",                  get(health::health_check))
        .route("/api/connection",              get(health::check_connection))
        .route("/api/people",                  get(people::get_people))
        .route("/api/people/{person_id}/thumbnail", get(people::get_person_thumbnail))
        .route("/api/graph/compute",           axum::routing::post(graph::compute_graph))
        .route("/api/graph/result",            get(graph::get_result))
        .route("/api/graph/cancel",            axum::routing::post(graph::cancel_graph))
        .route("/api/ws",                      get(ws::ws_handler))
        .route("/api/upload",                  axum::routing::post(upload::upload_to_immich))
        .route("/api/config",
            get(config::get_config).put(config::update_config))
        .route("/api/config/defaults",         get(config::get_config_defaults))
        // Serve built frontend, falling back to index.html so SPA deep links work.
        .fallback_service(ServeDir::new("frontend/dist").fallback(get(serve_index)))
        .with_state(state)
}

async fn serve_index() -> impl IntoResponse {
    match tokio::fs::read_to_string("frontend/dist/index.html").await {
        Ok(html) => Html(html).into_response(),
        Err(_) => Html(
            r#"<!DOCTYPE html><html><body style="font-family:sans-serif;background:#050506;color:#EDEDEF;display:grid;place-items:center;height:100vh;margin:0">
<div style="text-align:center"><h1>Frontend not built</h1>
<p>Run <code>cd frontend &amp;&amp; npm install &amp;&amp; npm run build</code></p></div></body></html>"#,
        ).into_response(),
    }
}
```

- [ ] **Step 2: Write `src/web/mod.rs`**

```rust
pub mod handlers;
pub mod state;

pub use handlers::create_router;
pub use state::AppState;
```

- [ ] **Step 3: Write `src/main.rs`**

```rust
use koram::{
    config::{Config, CONFIG_PATH},
    web::{self, AppState},
};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "koram=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = load_config()?;
    let state = AppState::new(config);
    let app = web::create_router(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], 5000));
    tracing::info!("koram listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn load_config() -> anyhow::Result<Config> {
    let p = std::path::Path::new(CONFIG_PATH);
    let cfg = if p.exists() {
        Config::from_file(p)?.with_env()
    } else {
        let default = Config::from_env();
        if let Err(e) = default.save_to_file(p) {
            tracing::warn!("Could not write default config to {}: {}", CONFIG_PATH, e);
        }
        default
    };
    if let Err(e) = cfg.validate() {
        tracing::warn!("Config incomplete: {}", e);
    }
    Ok(cfg)
}
```

- [ ] **Step 4: Build the full backend**

```bash
cargo build --release 2>&1 | tail -3
```

Expected: clean release build.

- [ ] **Step 5: Smoke test the server (background)**

```bash
mkdir -p config cache
IMMICH_API_KEY=demo IMMICH_BASE_URL=http://localhost:1 cargo run &
sleep 2
curl -s http://localhost:5000/api/health
kill %1 2>/dev/null || true
```

Expected: `{"ok":true}`.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/web/mod.rs src/web/handlers/mod.rs Cargo.lock
git commit -m "feat(web): wire up router and main; smoke-tested /api/health"
```

---

## Task 18: Frontend tokens + global CSS + fonts

**Files:**
- Replace: `frontend/src/styles/global.css`
- Modify: `frontend/index.html`

- [ ] **Step 1: Replace `frontend/src/styles/global.css`**

```css
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600&family=JetBrains+Mono:wght@400;500&display=swap');

:root {
  --bg-deep: #020203;
  --bg-base: #050506;
  --bg-elevated: #0a0a0c;
  --surface: rgba(255, 255, 255, 0.04);
  --surface-hover: rgba(255, 255, 255, 0.07);
  --border: rgba(255, 255, 255, 0.08);
  --foreground: #EDEDEF;
  --foreground-muted: #8A8F98;
  --accent: #7C3AED;
  --accent-glow: rgba(124, 58, 237, 0.20);
  --edge: #0891B2;
  --destructive: #DC2626;
  --warning: #F59E0B;
  --radius: 16px;
  --radius-control: 8px;
  --easing: cubic-bezier(0.16, 1, 0.3, 1);
  --font-sans: 'Inter', system-ui, sans-serif;
  --font-mono: 'JetBrains Mono', ui-monospace, monospace;
}

* { box-sizing: border-box; }

html, body, #app {
  margin: 0;
  height: 100%;
  background: var(--bg-base);
  color: var(--foreground);
  font-family: var(--font-sans);
  font-size: 14px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
}

button {
  font-family: inherit;
  font-size: inherit;
  background: var(--surface);
  color: var(--foreground);
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  padding: 8px 12px;
  cursor: pointer;
  transition: background 200ms var(--easing), border-color 200ms var(--easing);
}
button:hover { background: var(--surface-hover); }
button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}
button.primary:hover { filter: brightness(1.1); }

input[type="date"], input[type="text"], input[type="number"] {
  font-family: var(--font-mono);
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  color: var(--foreground);
  padding: 6px 10px;
  font-size: 13px;
}
input:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }

.mono { font-family: var(--font-mono); }
.muted { color: var(--foreground-muted); }

@media (prefers-reduced-motion: reduce) {
  * { transition-duration: 0ms !important; animation-duration: 0ms !important; }
}
```

- [ ] **Step 2: Verify build**

```bash
cd /home/beto/code/koram/frontend && npm run build
```

Expected: build succeeds; CSS bundled.

- [ ] **Step 3: Commit**

```bash
cd /home/beto/code/koram
git add frontend/src/styles/global.css
git commit -m "feat(frontend): design tokens and global styles"
```

---

## Task 19: API client, constants, utils

**Files:**
- Create: `frontend/src/lib/constants.js`
- Create: `frontend/src/lib/api.js`
- Create: `frontend/src/lib/utils.js`

- [ ] **Step 1: Write `frontend/src/lib/constants.js`**

```js
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
```

- [ ] **Step 2: Write `frontend/src/lib/api.js`**

```js
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
```

- [ ] **Step 3: Write `frontend/src/lib/utils.js`**

```js
export function debounce(fn, ms = 300) {
  let t;
  return (...args) => { clearTimeout(t); t = setTimeout(() => fn(...args), ms); };
}

export function formatCount(n) {
  return new Intl.NumberFormat().format(n);
}

export function todayIso() {
  return new Date().toISOString().slice(0, 10);
}

export function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
```

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/constants.js frontend/src/lib/api.js frontend/src/lib/utils.js
git commit -m "feat(frontend): API client, storage keys, util helpers"
```

---

## Task 20: Store + persistence (Svelte 5 runes)

**Files:**
- Create: `frontend/src/lib/store.js`
- Create: `frontend/src/lib/persistence.js`

- [ ] **Step 1: Write `frontend/src/lib/store.js`**

```js
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
```

- [ ] **Step 2: Write `frontend/src/lib/persistence.js`**

```js
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
```

> **Note:** `trackPersistence` uses `$effect`, which is only valid inside a component. Call it from `App.svelte`'s top-level `<script>` (Task 34).

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/store.js frontend/src/lib/persistence.js
git commit -m "feat(frontend): runes-based settings/graph stores + localStorage persistence"
```

---

## Task 21: ConnectionStatus component

**Files:**
- Create: `frontend/src/lib/components/ConnectionStatus.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script>
  import { getConnection } from '../api.js';

  let { } = $props();

  let status = $state('checking');   // 'checking' | 'ok' | 'error'
  let version = $state(null);
  let error = $state(null);

  async function poll() {
    try {
      const r = await getConnection();
      status = r.ok ? 'ok' : 'error';
      version = r.immich_version;
      error = r.error;
    } catch (e) {
      status = 'error';
      error = e.message;
    }
  }

  $effect(() => {
    poll();
    const id = setInterval(poll, 30_000);
    return () => clearInterval(id);
  });
</script>

<span class="dot {status}" title={status === 'ok' ? `Immich ${version}` : (error ?? 'Checking…')}></span>

<style>
  .dot {
    display: inline-block;
    width: 8px; height: 8px;
    border-radius: 50%;
    background: var(--foreground-muted);
    transition: background 200ms var(--easing);
  }
  .dot.ok { background: #22C55E; }
  .dot.error { background: var(--destructive); }
  .dot.checking { background: var(--warning); }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/ConnectionStatus.svelte
git commit -m "feat(frontend): ConnectionStatus indicator with polling"
```

---

## Task 22: TopBar component

**Files:**
- Create: `frontend/src/lib/components/TopBar.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script>
  import ConnectionStatus from './ConnectionStatus.svelte';
  import { graphState, settings } from '../store.js';
  import { formatCount } from '../utils.js';

  let { onToggleDrawer, canvasContainer = null } = $props();

  let visible = $state(true);
  let hideTimer = null;

  function bump() {
    visible = true;
    if (hideTimer) clearTimeout(hideTimer);
    hideTimer = setTimeout(() => { visible = false; }, 3000);
  }

  $effect(() => {
    bump();
    // Spec: "Auto-hides after 3s of cursor inactivity over the canvas".
    // Bind to the canvas container if provided, else fall back to window.
    const target = canvasContainer ?? window;
    target.addEventListener('mousemove', bump);
    window.addEventListener('keydown', bump);
    return () => {
      target.removeEventListener('mousemove', bump);
      window.removeEventListener('keydown', bump);
      if (hideTimer) clearTimeout(hideTimer);
    };
  });

  const computing = $derived(graphState.status === 'computing');

  const counter = $derived.by(() => {
    if (computing) {
      const { processed, total, currentPersonName } = graphState.progress;
      return `Fetching · ${currentPersonName ?? '…'} · ${processed}/${total}`;
    }
    if (!graphState.result) return '';
    const nodes = graphState.result.people.length;
    const edges = graphState.result.pairs.filter(p => p.count >= settings.minEdgeWeight).length;
    return `${formatCount(nodes)} nodes · ${formatCount(edges)} edges`;
  });

  const progressPct = $derived.by(() => {
    if (!computing) return 0;
    const { processed, total } = graphState.progress;
    if (!total) return 0;
    return Math.min(100, (processed / total) * 100);
  });
</script>

<div class="bar" class:visible aria-hidden={!visible}>
  <button class="hamburger" onclick={onToggleDrawer} aria-label="Open settings">
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5">
      <line x1="3" y1="6"  x2="17" y2="6"  />
      <line x1="3" y1="10" x2="17" y2="10" />
      <line x1="3" y1="14" x2="17" y2="14" />
    </svg>
  </button>
  <div class="counter mono">{counter}</div>
  <div class="status"><ConnectionStatus /></div>

  {#if computing}
    <div class="progress" aria-hidden="true">
      <div class="progress-fill" style:width="{progressPct}%"></div>
    </div>
  {/if}
</div>

<style>
  .bar {
    position: fixed; top: 0; left: 0; right: 0;
    height: 44px;
    display: flex; align-items: center;
    padding: 0 12px;
    gap: 12px;
    background: rgba(10, 10, 12, 0.6);
    backdrop-filter: blur(20px);
    border-bottom: 1px solid var(--border);
    z-index: 10;
    transition: opacity 250ms var(--easing), transform 250ms var(--easing);
    opacity: 0;
    transform: translateY(-8px);
    pointer-events: none;
  }
  .bar.visible { opacity: 1; transform: translateY(0); pointer-events: auto; }
  .bar:focus-within { opacity: 1; transform: translateY(0); pointer-events: auto; }

  .hamburger {
    width: 36px; height: 36px;
    display: grid; place-items: center;
    background: transparent;
    border: none;
    color: var(--foreground);
  }
  .hamburger:hover { background: var(--surface); }

  .counter {
    flex: 1;
    text-align: center;
    font-size: 12px;
    color: var(--foreground-muted);
  }

  .status { display: flex; align-items: center; }

  .progress {
    position: absolute;
    left: 0; right: 0; bottom: -1px;
    height: 2px;
    background: transparent;
  }
  .progress-fill {
    height: 100%;
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent-glow);
    transition: width 200ms var(--easing);
  }
</style>
```

> **Note for App.svelte (Task 34):** pass the canvas wrapper element so the auto-hide tracks canvas inactivity, not whole-window mouse motion: `<TopBar onToggleDrawer={toggleDrawer} canvasContainer={canvasWrapEl} />`. Bind the wrapper with `bind:this={canvasWrapEl}` (declared as `$state(null)`).

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/TopBar.svelte
git commit -m "feat(frontend): TopBar with hamburger, live counter, auto-hide"
```

---

## Task 23: PersonRow component

**Files:**
- Create: `frontend/src/lib/components/PersonRow.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script>
  import { API } from '../constants.js';
  import { settings } from '../store.js';
  import { formatCount } from '../utils.js';

  let { person } = $props(); // { id, name, total }

  const checked = $derived(settings.selected.has(person.id));
  const isNamed = $derived(!!person.name);
  const override = $derived(settings.perPersonOverrides[person.id] ?? null);

  function toggle() { settings.toggleSelected(person.id); }

  function setMode(mode) {
    settings.setOverride(person.id, mode === settings.displayMode ? null : mode);
  }
</script>

<div class="row" class:checked>
  <label>
    <input type="checkbox" {checked} onchange={toggle} />
    <img class="avatar" src={API.personThumb(person.id)} alt="" loading="lazy" />
    <span class="name" class:muted={!isNamed}>
      {person.name ?? `(unnamed #${person.id.slice(0, 4)})`}
    </span>
    <span class="count mono muted">{formatCount(person.total ?? 0)}</span>
  </label>
  {#if checked && isNamed}
    <div class="mode">
      <button
        class:active={(override ?? settings.displayMode) === 'thumbnail'}
        onclick={() => setMode('thumbnail')}
        title="Show face thumbnail">thumb</button>
      <button
        class:active={(override ?? settings.displayMode) === 'name'}
        onclick={() => setMode('name')}
        title="Show name only">name</button>
    </div>
  {/if}
</div>

<style>
  .row {
    border-bottom: 1px solid var(--border);
    padding: 8px 12px;
  }
  label {
    display: grid;
    grid-template-columns: 18px 32px 1fr auto;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }
  .avatar {
    width: 32px; height: 32px;
    border-radius: 50%;
    object-fit: cover;
    background: var(--surface);
  }
  .name { font-size: 13px; }
  .count { font-size: 12px; }
  .mode {
    display: flex; gap: 4px;
    margin-left: 60px;
    margin-top: 6px;
  }
  .mode button {
    padding: 2px 8px;
    font-size: 11px;
    text-transform: lowercase;
    background: transparent;
  }
  .mode button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/PersonRow.svelte
git commit -m "feat(frontend): PersonRow with checkbox, thumbnail, name, count, override toggles"
```

---

## Task 24: PeopleList component (with simple virtualization)

**Files:**
- Create: `frontend/src/lib/components/PeopleList.svelte`

A from-scratch virtualization is simpler than another dependency. We render only the rows whose vertical position intersects the viewport.

- [ ] **Step 1: Write the component**

```svelte
<script>
  import PersonRow from './PersonRow.svelte';
  import { settings } from '../store.js';

  let { people } = $props();   // Array<{id, name, total}>

  const ROW_HEIGHT = 56;       // 32 avatar + 24 vertical padding/borders

  let containerEl = $state(null);
  let scrollTop = $state(0);
  let viewportH = $state(400);

  function onScroll() { if (containerEl) scrollTop = containerEl.scrollTop; }

  $effect(() => {
    if (!containerEl) return;
    const ro = new ResizeObserver(() => { viewportH = containerEl.clientHeight; });
    ro.observe(containerEl);
    return () => ro.disconnect();
  });

  const filtered = $derived.by(() => {
    const q = settings.search.trim().toLowerCase();
    return people.filter(p => {
      if (!settings.showUnnamed && !p.name) return false;
      if (q && !(p.name ?? '').toLowerCase().includes(q)) return false;
      return true;
    });
  });

  const startIdx = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - 4));
  const endIdx   = $derived(Math.min(filtered.length, Math.ceil((scrollTop + viewportH) / ROW_HEIGHT) + 4));
  const padTop    = $derived(startIdx * ROW_HEIGHT);
  const padBottom = $derived((filtered.length - endIdx) * ROW_HEIGHT);
  const visible   = $derived(filtered.slice(startIdx, endIdx));

  function selectAll()   { settings.setSelected([...settings.selected, ...filtered.map(p => p.id)]); }
  function selectNone()  { const ids = new Set(filtered.map(p => p.id)); settings.setSelected([...settings.selected].filter(id => !ids.has(id))); }
</script>

<div class="header">
  <div class="search-row">
    <input type="text" placeholder="🔍 Search…" bind:value={settings.search} />
    <span class="count mono muted">{filtered.length}</span>
  </div>
  <label class="check-line">
    <input type="checkbox" bind:checked={settings.showUnnamed} />
    Show unnamed faces
  </label>
  <div class="select-actions">
    <button onclick={selectAll}>Select all</button>
    <button onclick={selectNone}>None</button>
  </div>
</div>

<div class="scroller" bind:this={containerEl} onscroll={onScroll}>
  <div style:height="{padTop}px"></div>
  {#each visible as p (p.id)}
    <PersonRow person={p} />
  {/each}
  <div style:height="{padBottom}px"></div>
</div>

<style>
  .header {
    display: flex; flex-direction: column; gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .search-row { display: flex; gap: 8px; align-items: center; }
  .search-row input { flex: 1; }
  .count { font-size: 12px; }
  .check-line { display: flex; gap: 8px; align-items: center; font-size: 13px; }
  .select-actions { display: flex; gap: 6px; }
  .select-actions button { padding: 4px 10px; font-size: 12px; }

  .scroller {
    flex: 1;
    overflow-y: auto;
    min-height: 200px;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/PeopleList.svelte
git commit -m "feat(frontend): PeopleList with search, select-all/none, simple windowed virtualization"
```

---

## Task 25: DateRange component

**Files:**
- Create: `frontend/src/lib/components/DateRange.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script>
  import { settings } from '../store.js';

  let allTime = $state(!settings.dateFrom && !settings.dateTo);

  function onAllTimeChange() {
    if (allTime) {
      settings.dateFrom = '';
      settings.dateTo = '';
    }
  }
</script>

<fieldset>
  <legend>Date range</legend>
  <div class="row">
    <label>From <input type="date" bind:value={settings.dateFrom} disabled={allTime} /></label>
    <label>To   <input type="date" bind:value={settings.dateTo}   disabled={allTime} /></label>
  </div>
  <label class="check-line">
    <input type="checkbox" bind:checked={allTime} onchange={onAllTimeChange} />
    All time
  </label>
</fieldset>

<style>
  fieldset { border: 1px solid var(--border); border-radius: var(--radius-control); padding: 10px 12px; margin: 0; }
  legend { padding: 0 6px; font-size: 12px; color: var(--foreground-muted); text-transform: uppercase; letter-spacing: 0.5px; }
  .row { display: flex; gap: 10px; flex-wrap: wrap; }
  .row label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--foreground-muted); }
  .check-line { display: flex; gap: 8px; align-items: center; margin-top: 8px; font-size: 13px; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/DateRange.svelte
git commit -m "feat(frontend): DateRange with all-time toggle"
```

---

## Task 26: DisplayControls component

**Files:**
- Create: `frontend/src/lib/components/DisplayControls.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script>
  import { settings } from '../store.js';

  let { selectedPeople } = $props();   // Array of currently selected person objects (for the unnamed-disable check)

  const anyUnnamed = $derived(selectedPeople.some(p => !p.name));

  function setMode(m) {
    if (m === 'name' && anyUnnamed) return;
    settings.displayMode = m;
  }
</script>

<fieldset>
  <legend>Display</legend>

  <div class="group">
    <span class="label">Node style</span>
    <label><input type="radio" name="ns" value="thumbnail"
      checked={settings.displayMode === 'thumbnail'}
      onchange={() => setMode('thumbnail')} /> Face thumbnail</label>
    <label class:disabled={anyUnnamed} title={anyUnnamed ? 'Disabled: some selected people are unnamed' : ''}>
      <input type="radio" name="ns" value="name"
        checked={settings.displayMode === 'name'}
        onchange={() => setMode('name')}
        disabled={anyUnnamed} /> Name only
    </label>
  </div>

  <div class="group">
    <span class="label">Edge weight</span>
    <label><input type="radio" name="ew" value="count"
      checked={settings.edgeMode === 'count'}
      onchange={() => settings.edgeMode = 'count'} /> Photo count</label>
    <label><input type="radio" name="ew" value="jaccard"
      checked={settings.edgeMode === 'jaccard'}
      onchange={() => settings.edgeMode = 'jaccard'} /> Jaccard similarity</label>
  </div>

  <div class="group">
    {#if settings.edgeMode === 'count'}
      <label class="label-inline">
        Min photos
        <input type="number" min="1" step="1" bind:value={settings.minEdgeWeight} />
      </label>
    {:else}
      <label class="label-inline">
        Min similarity
        <input type="range" min="0" max="1" step="0.01" bind:value={settings.minEdgeWeight} />
        <span class="mono">{Number(settings.minEdgeWeight).toFixed(2)}</span>
      </label>
    {/if}
  </div>
</fieldset>

<style>
  fieldset { border: 1px solid var(--border); border-radius: var(--radius-control); padding: 10px 12px; margin: 0; }
  legend { padding: 0 6px; font-size: 12px; color: var(--foreground-muted); text-transform: uppercase; letter-spacing: 0.5px; }
  .group { margin-top: 8px; display: flex; flex-direction: column; gap: 4px; }
  .label { font-size: 12px; color: var(--foreground-muted); }
  .label-inline { display: flex; gap: 8px; align-items: center; font-size: 13px; }
  label { font-size: 13px; display: flex; gap: 6px; align-items: center; }
  label.disabled { color: var(--foreground-muted); cursor: not-allowed; }
  input[type="number"] { width: 70px; }
  input[type="range"] { flex: 1; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/DisplayControls.svelte
git commit -m "feat(frontend): DisplayControls with mode-aware min-weight input"
```

---

## Task 27: SettingsDrawer component

**Files:**
- Create: `frontend/src/lib/components/SettingsDrawer.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script>
  import DateRange from './DateRange.svelte';
  import DisplayControls from './DisplayControls.svelte';
  import PeopleList from './PeopleList.svelte';
  import { settings, graphState } from '../store.js';
  import { computeGraph, cancelGraph } from '../api.js';

  let { people, setLastJobKey } = $props();   // all people from /api/people; lifecycle hook from App

  const selectedPeople = $derived(people.filter(p => settings.selected.has(p.id)));
  const peopleIdSet = $derived(new Set(people.map(p => p.id)));

  // Cached people that are no longer in /api/people (merged or deleted in Immich).
  const missingFromImmich = $derived.by(() => {
    if (!graphState.result) return [];
    return graphState.result.people
      .map(p => p.id)
      .filter(id => !peopleIdSet.has(id));
  });

  const isStale = $derived.by(() => {
    if (!graphState.result) return false;
    if (missingFromImmich.length > 0) return true;
    const r = graphState.result;
    const ids = [...settings.selected].sort().join(',');
    const cachedIds = r.people.map(p => p.id).sort().join(',');
    return ids !== cachedIds || (r.from ?? '') !== settings.dateFrom || (r.to ?? '') !== settings.dateTo;
  });

  const staleReason = $derived.by(() => {
    if (missingFromImmich.length > 0) return 'Some people were removed from Immich';
    if (isStale) return 'Settings changed since last refresh';
    return '';
  });

  async function onRefresh() {
    if (graphState.status === 'computing') {
      await cancelGraph();
      return;
    }
    if (settings.selected.size === 0) return;
    graphState.error = null;
    graphState.status = 'computing';
    try {
      const res = await computeGraph({
        person_ids: [...settings.selected],
        from: settings.dateFrom || null,
        to: settings.dateTo || null,
        force: true,
      });
      // The HTTP response returns immediately; the actual result lands via WS → App.svelte.
      // Only short-circuit when the backend served from cache.
      setLastJobKey?.(res.key);
      if (res.cached && res.result) {
        graphState.result = res.result;
        graphState.status = 'ready';
      }
    } catch (e) {
      graphState.error = e.message;
      graphState.status = 'error';
    }
  }

  function onClose() { settings.drawerOpen = false; }

  function onKey(e) { if (e.key === 'Escape') settings.drawerOpen = false; }
</script>

<svelte:window onkeydown={onKey} />

{#if settings.drawerOpen}
  <aside class="drawer">
    <div class="head">
      <h2>Settings</h2>
      <button class="close" onclick={onClose} aria-label="Close settings">✕</button>
    </div>

    <div class="body">
      <DateRange />
      <DisplayControls {selectedPeople} />

      <fieldset class="people">
        <legend>People</legend>
        <PeopleList {people} />
      </fieldset>
    </div>

    <div class="foot">
      <button class="primary" onclick={onRefresh} title={staleReason}>
        {#if graphState.status === 'computing'}
          Cancel
        {:else if isStale}
          <span class="stale-dot" aria-hidden="true"></span>
          ↻ Refresh ({staleReason || 'stale'})
        {:else}
          ↻ Refresh from Immich
        {/if}
      </button>
    </div>
  </aside>
{/if}

<style>
  .drawer {
    position: fixed; top: 0; right: 0; bottom: 0;
    width: 380px;
    display: flex; flex-direction: column;
    background: rgba(10, 10, 12, 0.82);
    backdrop-filter: blur(20px);
    border-left: 1px solid var(--border);
    z-index: 20;
    animation: slide 250ms var(--easing);
  }
  @keyframes slide { from { transform: translateX(20px); opacity: 0; } to { transform: none; opacity: 1; } }

  .head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }
  .head h2 { margin: 0; font-size: 14px; font-weight: 500; letter-spacing: 0.3px; }
  .close { width: 28px; height: 28px; padding: 0; background: transparent; border: none; }
  .close:hover { background: var(--surface); }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex; flex-direction: column; gap: 12px;
  }
  .people { padding: 0; }
  .people legend { padding: 0 6px; margin-left: 8px; font-size: 12px; color: var(--foreground-muted); text-transform: uppercase; letter-spacing: 0.5px; }

  .foot { border-top: 1px solid var(--border); padding: 12px; }
  .foot .primary { width: 100%; display: inline-flex; align-items: center; justify-content: center; gap: 8px; }
  .stale-dot {
    display: inline-block;
    width: 8px; height: 8px;
    border-radius: 50%;
    background: var(--warning);
    box-shadow: 0 0 6px var(--warning);
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/SettingsDrawer.svelte
git commit -m "feat(frontend): SettingsDrawer with WS-driven compute + stale-aware refresh"
```

---

## Task 28: graph/jaccard.js (TDD)

**Files:**
- Create: `frontend/src/lib/graph/jaccard.js`
- Test: `frontend/src/lib/graph/jaccard.test.js`

- [ ] **Step 1: Write the failing test** in `frontend/src/lib/graph/jaccard.test.js`

```js
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
```

- [ ] **Step 2: Run, expect failure**

```bash
cd /home/beto/code/koram/frontend && npx vitest run src/lib/graph/jaccard.test.js
```

Expected: FAIL — module not found.

- [ ] **Step 3: Write `frontend/src/lib/graph/jaccard.js`**

```js
export function jaccard(pairCount, totalA, totalB) {
  const denom = totalA + totalB - pairCount;
  if (denom <= 0) return 0;
  return pairCount / denom;
}
```

- [ ] **Step 4: Run, expect pass**

```bash
cd /home/beto/code/koram/frontend && npx vitest run src/lib/graph/jaccard.test.js
```

Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /home/beto/code/koram
git add frontend/src/lib/graph/jaccard.js frontend/src/lib/graph/jaccard.test.js
git commit -m "feat(graph): jaccard similarity with tests"
```

---

## Task 29: graph/force.js (TDD-light)

**Files:**
- Create: `frontend/src/lib/graph/force.js`
- Test: `frontend/src/lib/graph/force.test.js`

- [ ] **Step 1: Write the failing test**

```js
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
```

- [ ] **Step 2: Run, expect failure**

```bash
cd /home/beto/code/koram/frontend && npx vitest run src/lib/graph/force.test.js
```

Expected: FAIL.

- [ ] **Step 3: Write `frontend/src/lib/graph/force.js`**

```js
import { forceSimulation, forceLink, forceManyBody, forceCenter, forceCollide } from 'd3-force';

export function computeNodeRadius(total) {
  const r = 12 + Math.sqrt(Math.max(0, total)) * 1.5;
  return Math.max(12, Math.min(40, r));
}

const MIN_LINK_STRENGTH = 0.05;

export function weightToStrength(weight, maxWeight) {
  if (maxWeight <= 0) return 0;
  return Math.min(1, Math.max(MIN_LINK_STRENGTH, weight / maxWeight));
}

/**
 * Build (or rebuild) a d3-force simulation for the given graph.
 * `nodes` are mutated in place by d3 (each gets x, y, vx, vy, fx, fy).
 * `edges` get `source`/`target` rebound to node references.
 */
export function buildSimulation(nodes, edges, width, height) {
  const maxWeight = edges.reduce((m, e) => Math.max(m, e.weight), 0);
  edges.forEach(e => { e.weightNorm = weightToStrength(e.weight, maxWeight); });

  return forceSimulation(nodes)
    .force('link', forceLink(edges)
      .id(d => d.id)
      .distance(d => 80 + 200 / (1 + d.weightNorm))
      .strength(d => d.weightNorm))
    .force('charge', forceManyBody().strength(-300).distanceMax(800))
    .force('center', forceCenter(width / 2, height / 2).strength(0.05))
    .force('collide', forceCollide(d => d.radius + 4));
}

/** Recompute weights on edges after switching count↔jaccard. Call before .alpha(0.6).restart(). */
export function reweight(edges) {
  const maxWeight = edges.reduce((m, e) => Math.max(m, e.weight), 0);
  edges.forEach(e => { e.weightNorm = weightToStrength(e.weight, maxWeight); });
}
```

- [ ] **Step 4: Run, expect pass**

```bash
cd /home/beto/code/koram/frontend && npx vitest run src/lib/graph/force.test.js
```

Expected: 6 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /home/beto/code/koram
git add frontend/src/lib/graph/force.js frontend/src/lib/graph/force.test.js
git commit -m "feat(graph): d3-force simulation builder with tested weight mapping"
```

---

## Task 30: graph/render-canvas.js

**Files:**
- Create: `frontend/src/lib/graph/render-canvas.js`

- [ ] **Step 1: Write the renderer**

```js
const NODE_RING_PX = 1;
const HOVER_HALO_PX = 16;

/**
 * Per-tick canvas draw.
 * scene = { nodes, edges, transform: {k, x, y}, hover: nodeId|null, locked: nodeId|null,
 *           imageCache: Map<id, HTMLImageElement>, displayMode: (id) => 'thumbnail'|'name',
 *           label: (id) => string }
 */
export function draw(ctx, width, height, scene) {
  const dpr = window.devicePixelRatio || 1;
  ctx.save();
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, width, height);
  ctx.translate(scene.transform.x, scene.transform.y);
  ctx.scale(scene.transform.k, scene.transform.k);

  drawEdges(ctx, scene);
  drawNodes(ctx, scene);

  ctx.restore();
}

function isHighlighted(scene, nodeId) {
  const focus = scene.locked ?? scene.hover;
  if (!focus) return true;
  if (focus === nodeId) return true;
  return scene.adjacency.get(focus)?.has(nodeId) ?? false;
}

function edgeIsHighlighted(scene, edge) {
  const focus = scene.locked ?? scene.hover;
  if (!focus) return true;
  return edge.source.id === focus || edge.target.id === focus;
}

function drawEdges(ctx, scene) {
  for (const e of scene.edges) {
    const w = e.weightNorm;
    const focused = edgeIsHighlighted(scene, e);
    ctx.globalAlpha = focused ? 0.4 + 0.6 * w : 0.1;
    ctx.lineWidth = 1 + 2 * w;
    ctx.strokeStyle = scene.edgeColor;
    ctx.beginPath();
    ctx.moveTo(e.source.x, e.source.y);
    ctx.lineTo(e.target.x, e.target.y);
    ctx.stroke();
  }
  ctx.globalAlpha = 1;
}

function drawNodes(ctx, scene) {
  for (const n of scene.nodes) {
    const focused = isHighlighted(scene, n.id);
    ctx.globalAlpha = focused ? 1 : 0.2;

    const mode = scene.displayMode(n.id);
    const r = n.radius;

    if (mode === 'name') {
      // Pill with text
      const label = scene.label(n.id);
      ctx.font = '500 13px Inter, sans-serif';
      const m = ctx.measureText(label);
      const padX = 8;
      const w = m.width + padX * 2;
      const h = 22;
      ctx.fillStyle = '#0a0a0c';
      ctx.strokeStyle = focused && (scene.locked === n.id || scene.hover === n.id) ? scene.accentColor : 'rgba(255,255,255,0.12)';
      ctx.lineWidth = 1;
      roundRect(ctx, n.x - w / 2, n.y - h / 2, w, h, 11);
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = '#EDEDEF';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(label, n.x, n.y);
    } else {
      // Circular thumbnail
      const img = scene.imageCache.get(n.id);
      ctx.save();
      ctx.beginPath();
      ctx.arc(n.x, n.y, r, 0, Math.PI * 2);
      ctx.closePath();
      ctx.clip();
      if (img && img.complete && img.naturalWidth > 0) {
        ctx.drawImage(img, n.x - r, n.y - r, r * 2, r * 2);
      } else {
        ctx.fillStyle = '#171939';
        ctx.fill();
      }
      ctx.restore();

      // Halo for hovered/locked
      if (scene.locked === n.id || scene.hover === n.id) {
        const grad = ctx.createRadialGradient(n.x, n.y, r, n.x, n.y, r + HOVER_HALO_PX);
        grad.addColorStop(0, scene.accentGlow);
        grad.addColorStop(1, 'rgba(0,0,0,0)');
        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.arc(n.x, n.y, r + HOVER_HALO_PX, 0, Math.PI * 2);
        ctx.fill();
      }

      // Ring
      ctx.lineWidth = NODE_RING_PX;
      ctx.strokeStyle = (scene.locked === n.id || scene.hover === n.id) ? scene.accentColor : 'rgba(255,255,255,0.12)';
      ctx.beginPath();
      ctx.arc(n.x, n.y, r, 0, Math.PI * 2);
      ctx.stroke();
    }
  }
  ctx.globalAlpha = 1;
}

function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y,     x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x,     y + h, r);
  ctx.arcTo(x,     y + h, x,     y,     r);
  ctx.arcTo(x,     y,     x + w, y,     r);
  ctx.closePath();
}

/**
 * Edge hit-test: return the edge whose perpendicular distance to (px, py) is < threshold,
 * with the smallest distance. Returns null otherwise.
 * (px, py) are in graph (untransformed) coordinates.
 */
export function hitTestEdge(edges, px, py, threshold = 6) {
  let best = null;
  let bestD = threshold;
  for (const e of edges) {
    const x1 = e.source.x, y1 = e.source.y, x2 = e.target.x, y2 = e.target.y;
    const dx = x2 - x1, dy = y2 - y1;
    const len2 = dx * dx + dy * dy;
    if (len2 === 0) continue;
    const t = Math.max(0, Math.min(1, ((px - x1) * dx + (py - y1) * dy) / len2));
    const cx = x1 + t * dx, cy = y1 + t * dy;
    const d = Math.hypot(px - cx, py - cy);
    if (d < bestD) { bestD = d; best = e; }
  }
  return best;
}
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/graph/render-canvas.js
git commit -m "feat(graph): canvas renderer with hover, halo, name-pill mode, edge hit-test"
```

---

## Task 31: GraphCanvas component (drag, zoom, pan, hover, click)

**Files:**
- Create: `frontend/src/lib/components/GraphCanvas.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script>
  import { onDestroy } from 'svelte';
  import { drag } from 'd3-drag';
  import { select } from 'd3-selection';
  import { zoom, zoomIdentity } from 'd3-zoom';
  import { buildSimulation, reweight, computeNodeRadius } from '../graph/force.js';
  import { draw, hitTestEdge } from '../graph/render-canvas.js';
  import { jaccard } from '../graph/jaccard.js';
  import { settings, graphState } from '../store.js';
  import { API } from '../constants.js';

  let canvasEl = $state(null);
  let width = $state(0);
  let height = $state(0);
  let transform = $state({ k: 1, x: 0, y: 0 });
  let hover = $state(null);
  let locked = $state(null);
  let tooltip = $state(null); // { x, y, text }

  let nodes = [];
  let edges = [];
  let adjacency = new Map();
  let imageCache = new Map();
  let simulation = null;

  const cssVar = (name) =>
    getComputedStyle(document.documentElement).getPropertyValue(name).trim();

  function buildEdges(result) {
    const peopleById = new Map(result.people.map(p => [p.id, p]));
    const idSet = new Set(result.people.map(p => p.id));
    const out = [];
    for (const pair of result.pairs) {
      if (!idSet.has(pair.a) || !idSet.has(pair.b)) continue;
      const a = peopleById.get(pair.a);
      const b = peopleById.get(pair.b);
      const j = jaccard(pair.count, a.total, b.total);
      const wDisplay = settings.edgeMode === 'jaccard' ? j : pair.count;
      if (wDisplay < settings.minEdgeWeight) continue;
      // Internal weight is scaled when in jaccard mode so the force math stays linear.
      const w = settings.edgeMode === 'jaccard' ? j * 1000 : pair.count;
      out.push({ source: pair.a, target: pair.b, weight: w, displayWeight: wDisplay, count: pair.count });
    }
    return out;
  }

  function rebuildAdjacency(eds) {
    const adj = new Map();
    for (const e of eds) {
      const sId = typeof e.source === 'object' ? e.source.id : e.source;
      const tId = typeof e.target === 'object' ? e.target.id : e.target;
      if (!adj.has(sId)) adj.set(sId, new Set());
      if (!adj.has(tId)) adj.set(tId, new Set());
      adj.get(sId).add(tId);
      adj.get(tId).add(sId);
    }
    return adj;
  }

  /** Full rebuild — called only when the *result* changes (new compute). Position-resetting. */
  function rebuildGraph() {
    const r = graphState.result;
    if (!r) { nodes = []; edges = []; adjacency = new Map(); return; }

    nodes = r.people.map(p => ({
      id: p.id,
      name: p.name,
      total: p.total,
      radius: computeNodeRadius(p.total),
    }));

    edges = buildEdges(r);
    adjacency = rebuildAdjacency(edges);

    if (simulation) simulation.stop();
    simulation = buildSimulation(nodes, edges, width || 800, height || 600)
      .alpha(1)
      .on('tick', render);

    // Pre-fetch thumbnails into the cache
    for (const n of nodes) {
      if (imageCache.has(n.id)) continue;
      const img = new Image();
      img.crossOrigin = 'anonymous';
      img.src = API.personThumb(n.id);
      imageCache.set(n.id, img);
    }
  }

  /** Edges-only rebind — called when min weight or edge mode changes.
      Preserves node positions; just swaps the link force's input. */
  function rebindEdges() {
    const r = graphState.result;
    if (!r || !simulation) return;
    edges = buildEdges(r);
    adjacency = rebuildAdjacency(edges);
    reweight(edges);
    simulation.force('link').links(edges);
    simulation.alpha(0.3).restart();
  }

  function displayMode(id) {
    const override = settings.perPersonOverrides[id];
    if (override) return override;
    const node = nodes.find(n => n.id === id);
    if (!node?.name) return 'thumbnail';
    return settings.displayMode;
  }

  function label(id) {
    const node = nodes.find(n => n.id === id);
    return node?.name ?? `#${id.slice(0, 4)}`;
  }

  function render() {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext('2d');
    draw(ctx, width, height, {
      nodes, edges, transform, hover, locked, adjacency, imageCache,
      displayMode, label,
      edgeColor: cssVar('--edge') || '#0891B2',
      accentColor: cssVar('--accent') || '#7C3AED',
      accentGlow: cssVar('--accent-glow') || 'rgba(124,58,237,0.20)',
    });
  }

  function onResize() {
    if (!canvasEl) return;
    width = canvasEl.clientWidth;
    height = canvasEl.clientHeight;
    const dpr = window.devicePixelRatio || 1;
    canvasEl.width = Math.floor(width * dpr);
    canvasEl.height = Math.floor(height * dpr);
    if (simulation) {
      simulation.force('center').x(width / 2).y(height / 2);
      simulation.alpha(0.3).restart();
    }
    render();
  }

  function clientToGraph(cx, cy) {
    const rect = canvasEl.getBoundingClientRect();
    const x = (cx - rect.left - transform.x) / transform.k;
    const y = (cy - rect.top  - transform.y) / transform.k;
    return [x, y];
  }

  function nearestNode(gx, gy) {
    // Search radius shrinks as we zoom in so the click-target stays consistent on screen.
    const radius = 30 / Math.max(0.2, transform.k);
    return simulation?.find(gx, gy, radius) ?? null;
  }

  let dragMoved = false;

  function onMouseMove(ev) {
    const [gx, gy] = clientToGraph(ev.clientX, ev.clientY);
    const n = nearestNode(gx, gy);
    if (n) {
      hover = n.id;
      tooltip = null;
      render();
      return;
    }
    hover = null;
    const e = hitTestEdge(edges, gx, gy, 6 / transform.k);
    if (e) {
      const wText = settings.edgeMode === 'jaccard'
        ? `${e.displayWeight.toFixed(2)} jaccard`
        : `${e.count} photos`;
      tooltip = {
        x: ev.clientX, y: ev.clientY,
        text: `${label(e.source.id)} ↔ ${label(e.target.id)} · ${wText}`,
      };
    } else {
      tooltip = null;
    }
    render();
  }

  function onClick(ev) {
    if (dragMoved) { dragMoved = false; return; }
    const [gx, gy] = clientToGraph(ev.clientX, ev.clientY);
    const n = nearestNode(gx, gy);
    locked = n ? n.id : null;
    render();
  }

  function onDblClick(ev) {
    const [gx, gy] = clientToGraph(ev.clientX, ev.clientY);
    const n = nearestNode(gx, gy);
    if (n) { n.fx = null; n.fy = null; simulation?.alpha(0.3).restart(); }
  }

  $effect(() => {
    if (!canvasEl) return;
    onResize();
    const ro = new ResizeObserver(onResize);
    ro.observe(canvasEl);
    window.addEventListener('resize', onResize);
    return () => {
      ro.disconnect();
      window.removeEventListener('resize', onResize);
    };
  });

  // Tracks which result the simulation was last *fully rebuilt* for. Stays in sync with
  // graphState.result through rebuildGraph(). The rebind effect uses it to skip the
  // duplicate fire that would otherwise race a fresh rebuildGraph() on first result load.
  let resultBuiltFor = null;

  $effect(() => {
    // React to graphState.result changes — full rebuild
    const r = graphState.result;
    rebuildGraph();
    resultBuiltFor = r;
  });

  $effect(() => {
    // Reweight on edge mode or min weight changes — preserves positions.
    // Skip when the result itself just changed (rebuildGraph handles weights).
    settings.edgeMode; settings.minEdgeWeight;
    if (graphState.result && simulation && resultBuiltFor === graphState.result) {
      rebindEdges();
    }
  });

  // Wire d3-zoom + d3-drag
  $effect(() => {
    if (!canvasEl) return;
    const sel = select(canvasEl);

    const zoomBehavior = zoom()
      .scaleExtent([0.2, 4])
      .on('zoom', (ev) => { transform = { k: ev.transform.k, x: ev.transform.x, y: ev.transform.y }; render(); });

    sel.call(zoomBehavior);

    const dragBehavior = drag()
      // Only claim the gesture when the cursor is over a node. Otherwise let zoom
      // handle the mousedown so empty-space drags pan the canvas.
      .filter((ev) => {
        if (ev.button !== undefined && ev.button !== 0) return false;
        const [gx, gy] = clientToGraph(ev.clientX, ev.clientY);
        return nearestNode(gx, gy) !== null;
      })
      .subject((ev) => {
        const [gx, gy] = clientToGraph(ev.sourceEvent.clientX, ev.sourceEvent.clientY);
        return nearestNode(gx, gy);
      })
      .on('start', (ev) => {
        if (!ev.subject) return;
        dragMoved = false;
        if (!ev.active) simulation.alphaTarget(0.3).restart();
        ev.subject.fx = ev.subject.x;
        ev.subject.fy = ev.subject.y;
      })
      .on('drag', (ev) => {
        if (!ev.subject) return;
        dragMoved = true;
        const [gx, gy] = clientToGraph(ev.sourceEvent.clientX, ev.sourceEvent.clientY);
        ev.subject.fx = gx;
        ev.subject.fy = gy;
      })
      .on('end', (ev) => {
        if (!ev.subject) return;
        if (!ev.active) simulation.alphaTarget(0);
        // Keep fx/fy → pinned. Double-click clears.
      });

    sel.call(dragBehavior);

    return () => {
      sel.on('.zoom', null);
      sel.on('.drag', null);
    };
  });

  onDestroy(() => { simulation?.stop(); });

  // Exposed via `bind:this` for ExportFab → snapshot of the live scene.
  export function getScene() {
    return {
      width, height,
      nodes, edges,
      displayMode, label,
      imageCache,
      colors: {
        edge: cssVar('--edge') || '#0891B2',
        text: cssVar('--foreground') || '#EDEDEF',
        surface: cssVar('--bg-elevated') || '#0a0a0c',
        border: 'rgba(255,255,255,0.12)',
      },
    };
  }
</script>

<canvas
  bind:this={canvasEl}
  onmousemove={onMouseMove}
  onclick={onClick}
  ondblclick={onDblClick}
></canvas>

{#if tooltip}
  <div class="tooltip" style:left="{tooltip.x + 12}px" style:top="{tooltip.y + 12}px">{tooltip.text}</div>
{/if}

<style>
  canvas {
    width: 100%; height: 100%;
    display: block;
    cursor: grab;
  }
  canvas:active { cursor: grabbing; }
  .tooltip {
    position: fixed;
    pointer-events: none;
    padding: 4px 8px;
    background: rgba(10, 10, 12, 0.92);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 12px;
    z-index: 30;
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/GraphCanvas.svelte
git commit -m "feat(frontend): GraphCanvas with d3-force, drag-pin, zoom, edge tooltip, getScene"
```

---

## Task 32: render-svg.js + png-export.js (TDD)

**Files:**
- Create: `frontend/src/lib/graph/render-svg.js`
- Create: `frontend/src/lib/graph/png-export.js`
- Test: `frontend/src/lib/graph/png-export.test.js`

- [ ] **Step 1: Write the failing test** — `png-export.test.js`

```js
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
```

- [ ] **Step 2: Run, expect failure**

```bash
cd /home/beto/code/koram/frontend && npx vitest run src/lib/graph/png-export.test.js
```

Expected: FAIL.

- [ ] **Step 3: Write `frontend/src/lib/graph/render-svg.js`**

```js
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
```

- [ ] **Step 4: Write `frontend/src/lib/graph/png-export.js`**

```js
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
```

- [ ] **Step 5: Run tests, expect pass**

```bash
cd /home/beto/code/koram/frontend && npx vitest run src/lib/graph/png-export.test.js
```

Expected: 1 test passes.

- [ ] **Step 6: Commit**

```bash
cd /home/beto/code/koram
git add frontend/src/lib/graph/render-svg.js frontend/src/lib/graph/png-export.js frontend/src/lib/graph/png-export.test.js
git commit -m "feat(graph): SVG snapshot renderer + PNG export pipeline"
```

---

## Task 33: ExportFab component

**Files:**
- Create: `frontend/src/lib/components/ExportFab.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script>
  import { exportPng } from '../graph/png-export.js';
  import { jaccard } from '../graph/jaccard.js';
  import { settings, graphState } from '../store.js';
  import { uploadToImmich } from '../api.js';
  import { downloadBlob } from '../utils.js';

  let { sceneSnapshot } = $props();   // () => { width, height, nodes, edges, displayMode, label, imageCache, colors }

  let open = $state(false);
  let toast = $state(null);
  let busy = $state(false);

  function showToast(msg, kind = 'ok') {
    toast = { msg, kind };
    setTimeout(() => { toast = null; }, 1500);
  }

  async function onPng() {
    if (busy) return;
    busy = true;
    try {
      const blob = await exportPng(sceneSnapshot());
      downloadBlob(blob, `koram-${Date.now()}.png`);
      showToast('PNG saved');
    } catch (e) { showToast(e.message, 'err'); }
    finally { busy = false; open = false; }
  }

  async function onImmich() {
    if (busy) return;
    busy = true;
    try {
      const blob = await exportPng(sceneSnapshot());
      const id = `koram-${Date.now()}`;
      await uploadToImmich(blob, id);
      showToast('Uploaded to Immich');
    } catch (e) {
      showToast(e.message, 'err');
      // Fallback: also offer the local download
      try {
        const blob = await exportPng(sceneSnapshot());
        downloadBlob(blob, `koram-${Date.now()}.png`);
      } catch {}
    }
    finally { busy = false; open = false; }
  }

  function onCsv() {
    if (!graphState.result) return;
    const r = graphState.result;
    const peopleById = new Map(r.people.map(p => [p.id, p]));
    const lines = ['person_a_id,person_a_name,person_b_id,person_b_name,photo_count,jaccard'];
    for (const pair of r.pairs) {
      const a = peopleById.get(pair.a);
      const b = peopleById.get(pair.b);
      if (!a || !b) continue;
      const j = jaccard(pair.count, a.total, b.total);
      const wDisp = settings.edgeMode === 'jaccard' ? j : pair.count;
      if (wDisp < settings.minEdgeWeight) continue;
      const cells = [
        pair.a, csvEscape(a.name ?? ''),
        pair.b, csvEscape(b.name ?? ''),
        String(pair.count), j.toFixed(4),
      ];
      lines.push(cells.join(','));
    }
    const blob = new Blob([lines.join('\n')], { type: 'text/csv' });
    downloadBlob(blob, `koram-${Date.now()}.csv`);
    showToast('CSV saved');
    open = false;
  }

  function csvEscape(s) {
    if (/[",\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
    return s;
  }

  function onKey(e) { if (e.key === 'Escape') open = false; }
</script>

<svelte:window onkeydown={onKey} />

<div class="fab" class:open>
  {#if open}
    <button class="action" onclick={onPng}      disabled={busy}>⤓ PNG</button>
    <button class="action" onclick={onCsv}      disabled={busy}>⤓ CSV</button>
    <button class="action" onclick={onImmich}   disabled={busy}>⇪ Immich</button>
  {/if}
  <button class="trigger" onclick={() => open = !open} aria-label="Export">
    {open ? '✕' : '⤓'}
  </button>
</div>

{#if toast}
  <div class="toast {toast.kind}">{toast.msg}</div>
{/if}

<style>
  .fab {
    position: fixed; right: 20px; bottom: 20px;
    display: flex; flex-direction: column; gap: 8px; align-items: flex-end;
    z-index: 25;
  }
  .trigger, .action {
    width: 48px; height: 48px;
    border-radius: 50%;
    background: rgba(10, 10, 12, 0.82);
    backdrop-filter: blur(20px);
    border: 1px solid var(--border);
    color: var(--foreground);
    font-size: 16px;
    box-shadow: 0 6px 20px rgba(0,0,0,0.4);
    transition: transform 200ms var(--easing);
  }
  .action {
    width: auto;
    height: 40px;
    border-radius: 20px;
    padding: 0 14px;
    font-size: 13px;
  }
  .trigger:hover, .action:hover { transform: translateY(-1px); }

  .toast {
    position: fixed; bottom: 88px; right: 20px;
    padding: 8px 14px;
    background: rgba(10, 10, 12, 0.92);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
    z-index: 26;
  }
  .toast.err { border-color: var(--destructive); color: var(--destructive); }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/ExportFab.svelte
git commit -m "feat(frontend): ExportFab cluster (PNG, CSV, Immich) with toasts"
```

---

## Task 34: App.svelte assembly + first browser test

**Files:**
- Replace: `frontend/src/App.svelte`

- [ ] **Step 1: Write `frontend/src/App.svelte`**

```svelte
<script>
  import { onMount } from 'svelte';
  import TopBar from './lib/components/TopBar.svelte';
  import SettingsDrawer from './lib/components/SettingsDrawer.svelte';
  import GraphCanvas from './lib/components/GraphCanvas.svelte';
  import ExportFab from './lib/components/ExportFab.svelte';
  import { settings, graphState } from './lib/store.js';
  import { hydrate, trackPersistence } from './lib/persistence.js';
  import { getPeople, getConnection, openProgressSocket } from './lib/api.js';
  import { API } from './lib/constants.js';

  let people = $state([]);
  let loadError = $state(null);
  let connectionError = $state(null);   // for the 401/unreachable banner
  let canvasComp = $state(null);
  let canvasWrapEl = $state(null);
  let lastJobKey = $state(null);

  hydrate(settings);
  trackPersistence(settings);

  // WS message handler — drives graphState lifecycle.
  // The HTTP /api/graph/compute now returns immediately; we wait for WS terminal status.
  function onProgress(p) {
    graphState.progress = {
      processed: p.processed,
      total: p.total,
      currentPersonName: p.current_person_name ?? null,
    };
    if (p.status === 'running') {
      graphState.status = 'computing';
      graphState.error = null;
    } else if (p.status === 'completed') {
      // `message` carries the cache key for *this* job. Strict equality with the
      // job we kicked off (lastJobKey) guards against stale terminal messages
      // from a previously-spawned background task delivering after we cancelled.
      const key = p.message;
      if (!key || key !== lastJobKey) return;
      fetch(API.graphResult(key))
        .then(r => r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`)))
        .then(result => {
          graphState.result = result;
          graphState.status = 'ready';
        })
        .catch(e => { graphState.error = e.message; graphState.status = 'error'; });
    } else if (p.status === 'cancelled') {
      graphState.status = 'idle';
      lastJobKey = null;
    } else if (p.status === 'error') {
      graphState.error = p.message ?? 'Compute failed';
      graphState.status = 'error';
      lastJobKey = null;
    }
  }

  onMount(async () => {
    try {
      const conn = await getConnection();
      if (!conn.ok) connectionError = conn.error ?? 'Immich is unreachable.';
    } catch (e) {
      connectionError = e.message;
    }

    if (!connectionError) {
      try {
        people = await getPeople();
      } catch (e) {
        loadError = e.message;
      }
    }

    const ws = openProgressSocket(onProgress);
    return () => ws.close();
  });

  function sceneSnapshot() {
    const r = graphState.result;
    if (!r || !canvasComp?.getScene) return { width: 0, height: 0, nodes: [], edges: [] };
    return canvasComp.getScene();
  }

  function toggleDrawer() { settings.drawerOpen = !settings.drawerOpen; }

  // Click-outside closes the drawer (spec line 339).
  function onCanvasMouseDown() {
    if (settings.drawerOpen) settings.drawerOpen = false;
  }

  // Expose the chosen job key so the WS handler can call /result with it.
  // SettingsDrawer sets this when the compute kicks off.
  function setLastJobKey(k) { lastJobKey = k; }

  const isEmpty = $derived(!graphState.result && graphState.status !== 'computing' && !graphState.error);

  // No-edges-after-compute overlay (spec line 432)
  const computedButEmpty = $derived(
    graphState.result && graphState.result.pairs.filter(p => p.count >= settings.minEdgeWeight).length === 0
  );
</script>

<TopBar onToggleDrawer={toggleDrawer} canvasContainer={canvasWrapEl} />

{#if connectionError}
  <div class="banner">
    Can't reach Immich. Check API key in <code>/app/config/koram.toml</code>.
    <span class="muted">({connectionError})</span>
  </div>
{/if}

<div class="canvas-wrap" bind:this={canvasWrapEl} onmousedown={onCanvasMouseDown}>
  {#if graphState.result && !computedButEmpty}
    <GraphCanvas bind:this={canvasComp} />
  {:else if computedButEmpty}
    <div class="centered hint">
      <p>No co-occurrences. Lower min weight or pick more people.</p>
      <button onclick={toggleDrawer}>Open settings</button>
    </div>
  {:else if loadError}
    <div class="centered err">Couldn't load people: {loadError}</div>
  {:else if graphState.error}
    <div class="centered err">{graphState.error}</div>
  {:else if isEmpty}
    <div class="centered hint">
      <p>Pick at least 2 people from the menu.</p>
      <button onclick={toggleDrawer}>Open settings</button>
    </div>
  {:else}
    <div class="centered">Computing…</div>
  {/if}
</div>

<SettingsDrawer {people} {setLastJobKey} />
<ExportFab {sceneSnapshot} />

<style>
  .canvas-wrap {
    position: fixed; inset: 0;
    background: var(--bg-deep);
  }
  .banner {
    position: fixed; top: 44px; left: 0; right: 0;
    background: rgba(220, 38, 38, 0.15);
    border-bottom: 1px solid var(--destructive);
    color: var(--foreground);
    padding: 8px 16px;
    font-size: 13px;
    z-index: 15;
  }
  .banner code { font-family: var(--font-mono); background: rgba(0,0,0,0.4); padding: 1px 6px; border-radius: 4px; }
  .centered {
    position: absolute; inset: 0;
    display: grid; place-items: center;
    text-align: center;
    color: var(--foreground-muted);
  }
  .centered.err { color: var(--destructive); }
  .centered button { margin-top: 12px; }
</style>
```

> **Note:** `GraphCanvas.getScene()` was already exported in Task 31 — `ExportFab` calls it via `canvasComp.getScene()`.

- [ ] **Step 2: Build the frontend**

```bash
cd /home/beto/code/koram/frontend && npm run build
```

Expected: build succeeds; `dist/` produced.

- [ ] **Step 3: Manual smoke test (golden path)**

In one terminal:

```bash
cd /home/beto/code/koram
mkdir -p config cache
IMMICH_API_KEY=$YOUR_KEY IMMICH_BASE_URL=$YOUR_URL cargo run
```

In another:

```bash
cd /home/beto/code/koram/frontend && npm run dev
```

Open `http://localhost:5173`. Verify:
- Top bar appears, hamburger opens drawer
- People list populates, search filters, checkboxes toggle
- Refresh button kicks off a compute → canvas shows nodes + edges (via WS progress + `GET /api/graph/result?key=…`)
- Drag a node — it pins; double-click — it unpins
- Switch Photo Count ↔ Jaccard — graph re-flows without refetch and *keeps node positions*
- Move the min-weight slider — graph filters live without resetting positions
- Export FAB: PNG downloads, CSV downloads, Immich upload appears in the "Koram Graphs" album
- Cancel mid-compute — progress stops, no partial result surfaced

If any of the above fails, fix in this task before committing.

- [ ] **Step 4: Commit**

```bash
cd /home/beto/code/koram
git add frontend/src/App.svelte
git commit -m "feat(frontend): App.svelte assembly with end-to-end golden path"
```

---

## Task 35: README + manual verification checklist

**Files:**
- Create: `koram/README.md`

- [ ] **Step 1: Write `koram/README.md`**

```markdown
# Koram — Face Co-Occurrence Graph for Immich

Interactive force-directed graph of who appears in photos with whom, sourced from your Immich library's face recognition data.

## Features

- Per-person sweep of `/search/metadata` to build a co-occurrence matrix
- Force-directed layout with edge weight = photos containing both people
- Toggle between raw photo count and Jaccard similarity (client-side)
- Drag to pin, double-click to unpin, scroll/pinch to zoom
- Per-face display override (thumbnail vs name)
- Export as PNG, CSV, or upload back to Immich into a dedicated "Koram Graphs" album

## Quick start

### Immich API key

Create one in Immich (Account → API Keys) with these permissions:

- `album.create`
- `album.read`
- `album.update`
- `asset.read`
- `asset.upload`
- `asset.view`
- `asset.download`
- `person.read`
- `server.about`

### Docker Compose

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

Then open `http://your-server:5001`.

### Volumes

| Path | Description |
|---|---|
| `/app/config` | `koram.toml` (auto-created on first run) |
| `/app/cache` | Co-occurrence result cache (one JSON per `(person_set, date_range)`) |

## Development

```bash
# Backend (port 5000)
IMMICH_API_KEY=xxx IMMICH_BASE_URL=http://your-server:2283 cargo run

# Frontend (port 5173, proxies /api to 5000)
cd frontend && npm install && npm run dev
```

## License

MIT.
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: README with setup, permissions, dev workflow"
```

---

## Final Verification Checklist

Run through this list before declaring done:

- [ ] `cargo test --lib` — all unit tests pass
- [ ] `cd frontend && npx vitest run` — all frontend tests pass
- [ ] `cd frontend && npm run build` — clean Vite build
- [ ] `cargo build --release` — clean release build
- [ ] `docker build -t koram .` — image builds end-to-end
- [ ] Manual golden path against your own Immich (5 people, drag, switch mode, export PNG, export CSV, upload to Immich)
- [ ] Edge cases tested: 0 selected, 1 selected, 100+ selected, all unnamed
- [ ] Reduced-motion: no jarring animations when `prefers-reduced-motion: reduce`
- [ ] Cancel mid-compute works; partial result not surfaced as final
- [ ] Date range change marks Refresh stale; Refresh discards old cache file

---

## Self-Review Notes

Plan revised after a critical review pass. Changes made:

- **Task 9** rewritten to use `stream::iter().buffer_unordered(8)` so the spec's concurrency cap is actually enforced (the prior `FuturesUnordered + tokio::spawn` pattern fanned out unbounded).
- **Task 14** rewritten so `compute_graph` spawns the job and returns immediately with `{cached, key}`. Progress and terminal status flow through `/api/ws`; the client then `GET /api/graph/result?key=…`. This is what makes `/api/graph/cancel` actually meaningful.
- **`bind:this` targets** in Tasks 24, 31, 34 declared as `$state(null)` per Svelte 5 runes mode requirements.
- **`getScene()`** promoted into Task 31 as a numbered step (was a deferred prose blockquote in Task 34).
- **Task 31** simulation lifecycle split: full `rebuildGraph()` only on new compute results; `rebindEdges()` on min-weight or edge-mode changes preserves node positions.
- **Task 31** also: zoom-aware hit-test radius (`30 / transform.k`), `ResizeObserver` on the canvas itself, click-vs-drag guard via `dragMoved`.
- **Task 17** restored SPA fallback (`ServeDir::new(...).fallback(get(serve_index))`).
- **Task 22** auto-hide now bound to the canvas wrapper (not the whole window) per spec line 334; added a 2px progress bar.
- **Task 29** floors link strength at 0.05 so tiny-weight edges still pull.
- **Task 27 (SettingsDrawer)** now sets `lastJobKey` for the WS handler, drives compute via the new fire-and-listen flow, and surfaces stale state when cached people no longer appear in `/api/people`.
- **Task 34 (App.svelte)** now hosts the WS lifecycle, 401 banner, click-outside-closes-drawer, no-edges overlay, and computed-but-empty branch.
- **Task 16** adds `/api/config/defaults`; Task 17 routes it.
- **Tasks 19, 20** add `search` text persistence.

Spec coverage map:

| Spec section | Tasks |
|---|---|
| Backend stack & folder layout | 1, 3 |
| TOML config + env | 4 |
| Immich client (people, search, thumbnail) | 5, 6 |
| Immich client (upload, album) | 7 |
| Cooccurrence cache | 8 |
| Cooccurrence compute (concurrency-capped) | 9 |
| Job state / cancellation | 10, 11 |
| API surface incl. backgrounded compute, defaults, SPA fallback | 12–17 |
| Visual style tokens & fonts | 18 |
| API client / persistence (incl. search) / store | 19–20 |
| TopBar w/ progress bar + canvas-bound auto-hide | 22 |
| ConnectionStatus | 21 |
| PeopleList, PersonRow, DateRange, DisplayControls | 23–26 |
| SettingsDrawer (WS-driven, stale detection, missing-from-Immich tooltip) | 27 |
| graph/jaccard.js | 28 |
| graph/force.js (floored strength) | 29 |
| graph/render-canvas.js + GraphCanvas (drag-pin, zoom-aware hit-test, edges-only rebind, getScene) | 30, 31 |
| graph/render-svg.js + png-export.js | 32 |
| ExportFab (PNG, CSV, Immich, fallback PNG on upload error) | 33 |
| App.svelte (401 banner, click-outside, no-edges overlay, WS handler) | 34 |
| README & permissions doc | 35 |

No "TBD" / "TODO" / "implement later" entries. Type names are consistent: `CoOccurrenceResult`, `PairCount`, `PersonNode`, `Progress`, `AppState`, `ImmichClient`, `Settings`, `GraphState`.
