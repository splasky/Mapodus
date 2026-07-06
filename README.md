# Google Maps to uMap

> [!WARNING]
> This project is still under development. The code may change frequently. Not recommended for production use.

Import your Google Maps saved places into [uMap](https://umap.openstreetmap.fr/) — as a web app or CLI tool.

## Features

- **Live Google Maps import** (via cookies) — fetch all saved lists and places directly
- **Google Takeout CSV import** — upload a Saved.csv export; missing coordinates are auto-extracted from URLs
- **Cookie-based enrichment** — optionally paste Google cookies to fill in addresses and missing data
- **uMap upload** — creates a new map (or one map per list) with bilingual properties (Chinese + English)
- **CLI mode** — headless conversion from CSV or GeoJSON to uMap

## Web UI

### Prerequisites

- Rust nightly toolchain
- Node.js 22+ and npm

### Setup

```bash
# Build the backend
cargo build --workspace

# Build the frontend
cd frontend && npm install && npm run build && cd ..

# Copy .env.example to .env and edit with your settings
# Optional: UMAP_DEFAULT_URL (default: https://umap.openstreetmap.fr/en/)
# Optional: GOOGLE_MAPS_API_KEY (resolve missing POI coordinates)
# Optional: DEV_MODE, DATABASE_URL

# Start the server
cargo run --bin web
```

Open **http://localhost:8900** in your browser.

### Usage flow

1. **Import** — choose **Google Takeout CSV** or **Live Google Maps import (cookies)**
2. **Select bookmarks** — pick which places to transfer
3. **Connect uMap** — enter your uMap instance URL and session cookies
4. **Transfer** — places are uploaded as markers to a new uMap map

## CLI

```
google-maps-to-umap [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-t, --takeout <FILE>` | Path to Google Takeout CSV |
| `-g, --geojson <FILE>` | Path to existing GeoJSON file |
| `-o, --output <FILE>` | Output GeoJSON file path (skip uMap upload) |
| `--umap-url <URL>` | uMap instance URL (default: `https://umap.openstreetmap.fr/en/`) |
| `--umap-map-id <ID>` | uMap map ID to upload to |
| `--umap-cookie <COOKIE>` | Session cookie (`sessionid=xxx; csrftoken=xxx`) |
| `--layer-name <NAME>` | Target layer name (default: `Google Maps Saved`) |

### Examples

```bash
# Convert CSV to GeoJSON locally
google-maps-to-umap --takeout ./Saved.csv --output ./places.geojson

# Convert and upload to uMap
google-maps-to-umap --takeout ./Saved.csv --umap-map-id 123456 --umap-cookie "sessionid=xxx; csrftoken=xxx"
```

## Authentication

### Google Maps cookies (live import)
1. Open [google.com/maps](https://www.google.com/maps) in your browser
2. Open Developer Tools → Application → Cookies
3. Copy all cookies as a string
4. Paste into the cookie textarea in the import step

### uMap cookies
1. Log in to your uMap instance
2. Copy the `sessionid` and `csrftoken` cookies
3. Enter them in the "Connect uMap" step

## Development

```bash
# Start the backend in watch mode
cargo watch -x run --bin web

# Start the frontend dev server (hot-reload)
cd frontend && npm run dev
```

The frontend dev server runs on port 5173 with API proxied to `localhost:8900`.

### Project structure

```
├── core/           # Core library (parsing, conversion, API client)
├── cli/            # CLI binary
├── web/            # Web server (Axum)
├── frontend/       # Svelte SPA
└── umap/           # uMap submodule (for local testing)
```

## License

Apache-2.0
