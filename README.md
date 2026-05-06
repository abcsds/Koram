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
