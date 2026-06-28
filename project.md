# google-maps-to-umap Architecture

## Overview

Convert saved places from **Google Maps** (Saved, Favorites, Want to go, Starred, custom lists) and **Apple Maps** (Favorites, Guides, custom lists) to **uMap**. A Rust workspace with three crates:

- **`core/`** — shared library (parsing, conversion, uMap API client)
- **`cli/`** — CLI binary
- **`web/`** — Axum web server + embedded Svelte SPA

```
User Google Account                         Apple Account
    │                                            │
    ├── Google Takeout CSV (per-list) ─┐          ├── Apple Data & Privacy JSON
    │                                  │          │
    ├── Google Takeout Saved           │          └── Apple Shortcuts GPX
    │   Places.json (GeoJSON) ─────── ┤               │
    │                                  │              │
    └── (future) Data                  │              │
        Portability API ───────────── ┘              │
                                       │             │
                          ┌────────────┴─────────────┘
                          │  core::parse_source()      │
                          │  → Vec<Place>               │
                          └────────────┬────────────────┘
                                       │
                          ┌────────────┴────────────┐
                          │  core::convert::to_umap_  │
                          │  geojson()                │
                          └────────────┬────────────┘
                                       │ GeoJSON
                                       ▼
                          ┌─────────────────────────┐
                          │  core::umap::UmapClient  │
                          │  ├── proxy_login()       │
                          │  ├── create_map()        │
                          │  ├── create_layer()      │
                          │  └── upload_geojson()    │
                          └──────────┬──────────────┘
                                     │ HTTP (reqwest)
                                     ▼
                          ┌─────────────────────────┐
                          │    uMap Django App       │
                          │  (local podman or remote)│
                          └─────────────────────────┘
```

## Workspace Structure

```
google-maps-to-umap/
├── Cargo.toml                    # [workspace] members = ["core", "cli", "web"]
│
├── core/                         # Library crate (shared logic)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # re-exports all modules
│       ├── place.rs              # Place struct (source-agnostic, replaces GooglePlace)
│       ├── google.rs             # GoogleTakeout parser (CSV per-list + Saved Places.json)
│       ├── apple_maps.rs         # Apple Maps parser (JSON export, GPX)
│       ├── convert.rs            # Converter::to_geojson, to_umap_geojson
│       ├── error.rs              # AppError
│       ├── google_maps_api.rs    # Phase A: Google Maps live API client (cookie-based)
│       └── umap/
│           ├── mod.rs
│           ├── auth.rs           # CookieAuth (sessionid + csrftoken)
│           ├── login.rs          # proxy_login(username, password) -> CookieAuth
│           └── upload.rs         # UmapClient: create_map (private), upload_geojson, etc.
│
├── cli/                          # CLI binary
│   ├── Cargo.toml                # depends on core + clap + tokio
│   └── src/
│       ├── main.rs
│       └── cli.rs
│
├── web/                          # Web server binary
│   ├── Cargo.toml                # depends on core + axum + oauth2 + tower-sessions + rust-embed
│   ├── build.rs                  # runs `npm run build` in frontend/ at build time
│   └── src/
│       ├── main.rs               # Axum server, router, embedded static files
│       └── api/
│           ├── mod.rs            # route tree
│           ├── auth.rs           # Google OAuth2 login/callback
│           ├── bookmarks.rs      # upload CSV/JSON/GPX, parse, list
│           ├── umap.rs           # uMap connect (proxy login), transfer
│           ├── google_import.rs  # Phase A: Google Maps API import handlers
│           └── errors.rs         # API error types
│
├── frontend/                     # Svelte SPA (built to web/static/)
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── svelte.config.js
│   └── src/
│       ├── main.ts
│       ├── App.svelte
│       ├── app.css
│       └── lib/
│           ├── api.ts            # fetch() wrapper for REST API
│           └── components/
│               ├── Login.svelte          # "Sign in with Google"
│               ├── Upload.svelte         # Drag & drop CSV/JSON/GPX + Google Import link
│               ├── GoogleImport.svelte   # Phase A: paste cookies → select lists
│               ├── Bookmarks.svelte      # Grouped by list, with checkboxes
│               ├── ConnectUmap.svelte    # URL + username/password form
│               └── Transfer.svelte       # Per-list layer upload, progress
│
├── umap/                         # uMap Django project (vendored, for podman compose)
│   └── docker-compose.yml        # 4 services: redis, postgis, app, nginx
│
└── examples/                     # Test data
    ├── 2026北海道.csv
    ├── 2026北海道_updated.csv
    ├── 2026北海道_umap.geojson
    ├── csv_to_geojson.py
    ├── process_csv_async.py
    ├── process_csv_playwright.py
    ├── google-takeout/            # Simulated Google Takeout export
    │   ├── Saved Places.json      # Maps (your places) — GeoJSON, starred only
    │   ├── Starred places.csv     # Saved → per-list CSV
    │   ├── Want to go.csv
    │   ├── Favorites.csv
    │   └── Tokyo 2025.csv         # Custom list
    └── apple-maps/                # Simulated Apple Maps export
        ├── Apple Maps Places.json # Apple Data & Privacy export
        └── favorites.gpx          # Apple Shortcuts GPX export
```

## Data Sources

### Google Maps — Two Export Paths

Google Takeout provides **two distinct export paths** for saved places:

| Takeout Product | Format | Contains | Coords? |
|----------------|--------|----------|---------|
| "Maps (your places)" | `Saved Places.json` (GeoJSON) | Starred places only | Yes (when available) |
| "Saved" | One CSV per list (filename = list name) | All lists (Starred, Want to go, Favorites, custom) | No (URL only) |

**CSV columns** (per-list, from "Saved" product):
| Column | Example |
|--------|---------|
| Title | Ichiran Ramen Shibuya |
| Note | Great solo booth ramen |
| URL | https://maps.google.com/?cid=... |
| Address | 1-22-7 Jinnan, Shibuya, Tokyo |

The **list name** is the CSV filename (e.g., `Starred places.csv`, `Want to go.csv`, `Tokyo 2025.csv`). Our parser:
- Accepts a **directory** of CSVs (one per list) or a single CSV
- Each CSV row represents a `Place` with `source: "google"` and `list` set from filename
- Missing coordinates → reverse-geocode the address row, or defer to user to enrich

### Google Maps — Saved Places.json (GeoJSON)

From "Maps (your places)" product:
```json
{
  "type": "FeatureCollection",
  "features": [{
    "type": "Feature",
    "geometry": {"type": "Point", "coordinates": [139.7, 35.7]},
    "properties": {
      "Title": "Ichiran Ramen",
      "Google Maps URL": "https://...",
      "Author Name": null
    }
  }]
}
```
- Only includes **starred places** (not custom lists)
- Properties: `Title`, `Google Maps URL`, `Author Name`
- Parser maps to `Place` with `source: "google"`, `list: "Starred places"`

### Apple Maps — Export Paths

Apple does not have a standard "export saved places" feature. Users can extract via:

1. **Apple Data & Privacy** (privacy.apple.com) → request Maps data → receive JSON
   - Expected JSON format (Apple internal schema, approximate):
     ```json
     [{
       "Place Name": "Joe's Coffee",
       "Address": "123 Main St, Portland, OR",
       "Telephone": "+1-555-0100",
       "Latitude": 45.515,
       "Longitude": -122.678,
       "Category": "Coffee Shop",
       "URL": "",              // Apple Maps URL or website
       "Created At": "2024-03-15T10:30:00Z",
       "Source App": "Maps",   // or "Safari", "Messages"
       "Favorite": true        // Is this a favorite?
     }]
     ```
   - Flexible key matching: `Place Name` / `Name` / `Title`, `Latitude` / `Lat`, etc.
   - Maps to `Place` with `source: "apple"`, `list` from collection name

2. **Apple Shortcuts** → GPX export
   - Standard GPX waypoints:
     ```xml
     <wpt lat="45.515" lon="-122.678">
       <name>Joe's Coffee</name>
       <desc>Coffee Shop - 123 Main St</desc>
     </wpt>
     ```
   - Maps to `Place` with `source: "apple"`, minimal metadata

3. **Screen scraping** / third-party tools (not natively supported)
   - Extensions can extract data from Apple Maps web interface `maps.apple.com`

## Core Data Model

```rust
pub struct Place {
    // Identity
    pub title: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,

    // Metadata
    pub notes: Option<String>,
    pub url: Option<String>,          // original maps URL
    pub tags: Vec<String>,
    pub rating: Option<f64>,
    pub website: Option<String>,
    pub description: Option<String>,

    // Source + list membership
    pub source: Source,               // Google | Apple
    pub list: Option<String>,         // "Favorites", "Want to go", custom name

    // Bilingual fields (Google Takeout only)
    pub original_name: Option<String>,
    pub english_name: Option<String>,
}

pub enum Source {
    Google,
    Apple,
}
```

Replaces the old `GooglePlace` struct. A parser registry maps file types to parsers:

```rust
pub fn parse_source(path: &Path) -> Result<Vec<Place>> {
    // 1. Detect file type (extension, content sniff)
    // 2. Route to appropriate parser:
    //    - .csv → GoogleTakeoutParser (single or dir)
    //    - .json → look for GeoJSON structure → GoogleSavedPlacesParser / AppleMapsParser
    //    - .gpx → GpxParser
    //    - directory → walk files, each parsed independently
}
```

## Google Maps List Awareness

### How lists are detected

1. **Per-list CSV files** (from Takeout "Saved" product)
   - Upload a **folder** (not single file) containing `Starred places.csv`, `Want to go.csv`, etc.
   - List name = filename minus extension
   - All CSVs parsed together → `Vec<Place>` with different `list` values

2. **Single "enriched" CSV** (custom, with coordinates)
   - Existing format: `標題,筆記,網址,標籤,留言,緯度,經度,...`
   - "Label" column (if present) maps to `list` field
   - If no Label column, `list` defaults to `"Saved"`

3. **Saved Places.json** (GeoJSON)
   - Only starred places → `list: "Starred places"`
   - Coordinates may be zero (`[0, 0]`) for some entries → flag for review

### How lists are used

- **CLI**: `--list "Favorites"` filter, upload only places from one list
- **CLI**: `--all-lists` → upload each list as a **separate layer** on the same map
- **Web UI**: Bookmarks grouped by list heading, toggle entire lists on/off
- **Upload**: Each list becomes a uMap layer (named after the list)

## Apple Maps Import

### Parser Design

`core/src/apple_maps.rs`:
- `parse_apple_json(path)` — flexible key matching for Apple's JSON export
- `parse_gpx(path)` — standard GPX waypoints with name/desc

Key matching (case-insensitive, multiple possible names):

| Field | Possible keys |
|-------|--------------|
| title | "Place Name", "Name", "Title", "name" |
| address | "Address", "address", "FullAddress" |
| latitude | "Latitude", "Lat", "lat", "LatitudeDeg" |
| longitude | "Longitude", "Lng", "lng", "lon", "LongitudeDeg" |
| url | "URL", "url", "Website", "Identifier" |
| notes | "Notes", "Note", "Description", "desc" |
| rating | "Rating", "Stars" |
| tags | "Category", "Type", "Tags" |
| list | "Collection", "List", "Group", "Guide" |

### Apple Maps → uMap specifics

- Description field: formatted as `🏛 {title}\n📍 {address}\n🔗 {url}\n📝 {notes}`
- No bilingual support (Apple Maps doesn't have this)
- GPX: basic import, name + description only

## CLI Changes

```
# Google Takeout CSV (single file, no list info)
cargo run -p cli -- --takeout "Saved places.csv"

# Google Takeout CSV directory (per-list files)
cargo run -p cli -- --takeout-dir ./google-takeout/

# Google Saved Places.json (GeoJSON)
cargo run -p cli -- --takeout-json "Saved Places.json"

# Filter by list
cargo run -p cli -- --takeout-dir ./google-takeout/ --list "Favorites"

# Upload each list as a separate uMap layer
cargo run -p cli -- --takeout-dir ./google-takeout/ --all-layers

# Apple Maps JSON export
cargo run -p cli -- --apple-maps "Apple Places.json"

# Apple Maps GPX
cargo run -p cli -- --apple-maps "favorites.gpx"

# Auto-detect file type
cargo run -p cli -- --import saved_places.json
  # → detects GeoJSON → GoogleSavedPlaces parser
  # → detects Apple JSON → AppleMaps parser
  # → detects CSV → GoogleTakeout parser
  # → detects GPX → GPX parser
```

## Map + Layer Strategy

```
uMap Map "My Saved Places"
├── Layer: "Favorites"        (10 places)
├── Layer: "Want to go"      (23 places)
├── Layer: "Tokyo 2025"      (15 places)
└── Layer: "Starred places"  (8 places)
```

- **Single map** with **one layer per list**
- CLI `--create-map "Trip to Japan"` creates the map, then creates one layer per list
- CLI `--umap-map-id` uploads to an existing map, adding layers for any new lists
- Existing layers with matching names are **updated** (not duplicated)
- Layer name = list name

## uMap Authentication: Proxy Login

Since we run a local uMap instance, users log in via proxy:

1. `GET {umap_url}/login/` → parse CSRF token from HTML form
2. `POST {umap_url}/login/` with `username`, `password`, `csrfmiddlewaretoken`
3. Follow redirect, capture `sessionid` cookie from `Set-Cookie`
4. `GET {umap_url}/` to verify session
5. Return `CookieAuth { session_id, csrf_token }`

Stored in server-side session, used for all subsequent uMap API calls.

## Map Privacy

Maps default to **DRAFT (private)** — `share_status=0`:

| Status | Value | Meaning |
|--------|-------|---------|
| DRAFT | 0 | Only owner can view/edit (private) |
| PUBLIC | 1 | Everyone can view |
| OPEN | 2 | Anyone with link |
| PRIVATE | 3 | Editors and team only |

Edit status set to `OWNER=3`.

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/auth/google` | Redirect to Google OAuth |
| `GET` | `/api/auth/google/callback` | OAuth callback → store identity |
| `GET` | `/api/auth/status` | Current user + uMap connection |
| `POST` | `/api/bookmarks/upload` | Accept CSV **directory** (zip), JSON, or GPX → parse → store |
| `GET` | `/api/bookmarks` | List parsed bookmarks grouped by list |
| `POST` | `/api/umap/connect` | Proxy login to uMap → store CookieAuth |
| `GET` | `/api/umap/status` | Check if uMap session is valid |
| `POST` | `/api/transfer` | Create private map + upload selected lists as layers |
| `POST` | `/api/google/import` | Import from Google Maps live API (cookies) → list lists |
| `POST` | `/api/google/confirm` | Confirm which lists to import → store as bookmarks |

### Bookmark list grouped response

```json
{
  "lists": [
    {
      "name": "Favorites",
      "count": 10,
      "places": [...]
    },
    {
      "name": "Want to go",
      "count": 23,
      "places": [...]
    }
  ],
  "source": "google"
}
```

## Frontend UI Flow

```
Login (Google OAuth)
    │
    ▼
Select Data Source ───┬── Google Takeout (CSV drag & drop)
                      ├── Google Maps API (paste cookies) ← Phase A
                      ├── Google Saved Places.json
                      ├── Apple Maps JSON
                      └── Apple Maps GPX
    │
    ▼
Select Lists + Bookmarks
    │
    ├── Grouped by list (expand/collapse)
    ├── Toggle entire lists on/off
    └── Individual checkboxes
    │
    ▼
Connect uMap (URL + username + password)
    │
    ▼
Transfer (progress bar → result link)
    ├── Create map
    ├── For each selected list:
    │   ├── Create layer (or update existing)
    │   └── Upload GeoJSON
    └── Show result: map URL
```

### Phase A: Google Maps API Import

```
Upload Step (choose path)
    │
    ├── Upload CSV → parse → bookmarks → select → connect → transfer
    │
    └── Import from Google Maps
        │
        ├── [GoogleImport] User pastes cookies (SAPISID, SID, HSID)
        │       ↓
        │   POST /api/google/import { cookies }
        │       ↓
        │   Backend calls GoogleMapsClient::collect_all()
        │   → fetches session token from maps.google.com
        │   → calls MAS API for list metadata
        │   → calls getList API for each list's places
        │   → stores GoogleSavedPlace[] in session.google_places
        │       ↓
        ├── [GoogleImport] Shows grouped lists with checkboxes
        │       ↓
        │   POST /api/google/confirm { selected_lists: [...] }
        │       ↓
        │   Backend filters google_places by list name
        │   → converts GoogleSavedPlace → GooglePlace
        │   → stores in session.bookmarks
        │   → clears session.google_places
        │       ↓
        └── → proceed to "Select Bookmarks" (step 3)
```

## Svelte Frontend

- Plain Svelte + Vite (no SvelteKit — no SSR needed)
- `svelte-spa-router` for client-side routing
- Built to static files, embedded in Rust binary via `rust-embed`
- No Node.js runtime in production
- Calls REST API at `/api/*`

## Session Structure

```rust
struct AppSession {
    google_user: Option<GoogleUser>,
    bookmarks: Option<Vec<GooglePlace>>,    // parsed/converted places
    selected_ids: Option<Vec<usize>>,
    umap_auth: Option<CookieAuth>,
    umap_url: Option<String>,
    google_places: Option<Vec<GoogleSavedPlace>>,  // raw Google API places (Phase A)
}
```

## Build Phases

### Phase 1: Core refactor + Google list support
1. Rename `GooglePlace` → `Place`, add `source` + `list` fields
2. Add `core/src/place.rs`
3. Update `google.rs` — multi-CSV directory support, "Saved Places.json" parser
4. Update `convert.rs` — accept `Place`, preserve list in layer name
5. Update `cli/` — new flags (`--takeout-dir`, `--list`, `--all-layers`)
6. Verify CLI tests pass

### Phase 2: Apple Maps import
7. Add `core/src/apple_maps.rs` — JSON parser with flexible key matching
8. Add GPX parser (or use a lightweight dep)
9. Update `parse_source()` for auto-detection
10. Test with sample Apple Maps JSON + GPX

### Phase 3: Web + frontend
11. Update `web/src/api/bookmarks.rs` — grouped response, multi-format accept
12. Update `frontend/` — grouped list UI, data source selection
13. Update transfer handler — per-list layer creation
14. End-to-end test with podman compose

### Phase A: Google Maps Live API import (cookie-based)
A. Add `core/src/google_maps_api.rs` — `ProtoWriter` protobuf encoder, `GoogleMapsClient`,
   `GoogleSavedPlace` / `GoogleList` types, `collect_all()` convenience method
B. Add `web/src/api/google_import.rs` — `POST /api/google/import` and `POST /api/google/confirm`
C. Update `web/src/session.rs` — add `google_places: Option<Vec<GoogleSavedPlace>>`
D. Create `frontend/src/lib/components/GoogleImport.svelte` — cookie paste → list checkboxes → confirm
E. Update `frontend/src/App.svelte` — add `google-import` step to the flow
F. Update `frontend/src/lib/components/Upload.svelte` — add "Import from Google Maps" button
G. Test with real Google cookies against a disposable Google account

## Test Plan

```bash
# Unit tests (core library)
cargo test -p core

# CLI: Google CSV directory
cargo run -p cli -- --takeout-dir examples/google-takeout/ \
  --umap-url "http://localhost:8000/en/" \
  --umap-cookie "sessionid=xxx; csrftoken=yyy" \
  --create-map "All Lists"

# CLI: filter by list
cargo run -p cli -- --takeout-dir examples/google-takeout/ \
  --list "Favorites"

# CLI: Apple Maps JSON
cargo run -p cli -- --apple-maps examples/apple-maps/Apple\ Maps\ Places.json

# CLI: Apple Maps GPX
cargo run -p cli -- --apple-maps examples/apple-maps/favorites.gpx

# Web server
cargo run -p web
# → http://localhost:8900

# uMap (local instance for testing)
podman compose -f umap/docker-compose.yml up -d
# → http://localhost:8000/en/
```

## Dependencies

### core/Cargo.toml
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
csv = "1"
geojson = { version = "0.24", features = ["geo-types"] }
reqwest = { version = "0.12", features = ["json", "multipart"] }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
uuid = { version = "1.23.3", features = ["v4"] }
clap = { version = "4", features = ["derive"] }
# For GPX parsing (Apple Maps)
quick-xml = "0.36"        # lightweight XML
```

### cli/Cargo.toml
```toml
[dependencies]
core = { path = "../core" }
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

### web/Cargo.toml
```toml
[dependencies]
core = { path = "../core" }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors"] }
tower-sessions = "0.13"
oauth2 = "5"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
rust-embed = "8"
mime_guess = "2"
reqwest = { version = "0.12", features = ["json", "multipart", "cookies"] }
```
