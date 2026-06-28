# google-maps-to-umap Architecture

## Overview

Convert Google Maps saved places to uMap. A Rust workspace with three crates:
- **`core/`** — shared library (parsing, conversion, uMap API client)
- **`cli/`** — CLI binary (unchanged UX from original)
- **`web/`** — Axum web server + embedded Svelte SPA

```
User Google Account
    │
    ├── Google Takeout CSV  ──▶  core::parse_takeout()
    │                             │
    └── (future) Data Portability API  ──▶  JSON
                                      │
                         ┌────────────┴────────────┐
                         │  core::convert::to_umap_ │
                         │  geojson()               │
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
│       ├── google.rs             # GooglePlace, parse_takeout (CSV/JSON)
│       ├── convert.rs            # Converter::to_geojson, to_umap_geojson
│       ├── error.rs              # AppError
│       └── umap/
│           ├── mod.rs
│           ├── auth.rs           # CookieAuth (sessionid + csrftoken)
│           ├── login.rs          # NEW: proxy_login(username, password) -> CookieAuth
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
│           ├── bookmarks.rs      # upload CSV, parse, list
│           ├── umap.rs           # uMap connect (proxy login), transfer
│           └── errors.rs         # API error types
│
├── frontend/                     # Svelte SPA (built to web/static/)
│   ├── package.json
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
│               ├── Upload.svelte         # Drag & drop CSV
│               ├── Bookmarks.svelte      # Checkbox list
│               ├── ConnectUmap.svelte    # URL + username/password form
│               └── Transfer.svelte       # Progress bar + result link
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
    └── process_csv_playwright.py
```

## Data Sources for Google Bookmarks

### Phase 1 (MVP): Google Takeout CSV Upload
- User downloads Takeout from [takeout.google.com](https://takeout.google.com)
- Uploads CSV file through web UI
- `core::parse_takeout()` parses into `Vec<GooglePlace>`
- Works immediately, no app verification needed

### Phase 2 (Future): Google Data Portability API
- OAuth scope: `dataportability.maps.starred_places`
- User clicks "Import from Google" → OAuth consent → archive initiates
- Call `InitiatePortabilityArchive` → poll → download signed URL → parse JSON
- Requires Google Cloud project + OAuth app verification (Restricted scope)

Both paths feed into the same bookmark selection UI.

## uMap Authentication: Proxy Login

Since we run a local uMap instance, users log in via proxy:

1. `GET {umap_url}/login/` → parse CSRF token from HTML form
2. `POST {umap_url}/login/` with `username`, `password`, `csrfmiddlewaretoken`
3. Follow redirect, capture `sessionid` cookie from `Set-Cookie`
4. `GET {umap_url}/` to confirm successful auth
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
| `GET` | `/api/auth/google/callback` | OAuth callback → store identity in session |
| `GET` | `/api/auth/status` | Current user + uMap connection state |
| `POST` | `/api/bookmarks/upload` | Accept CSV → parse → store in session → return list |
| `GET` | `/api/bookmarks` | List parsed bookmarks from session |
| `POST` | `/api/umap/connect` | Proxy login to uMap → store CookieAuth |
| `GET` | `/api/umap/status` | Check if uMap session is valid |
| `POST` | `/api/transfer` | Create private map + upload selected bookmarks |

## Frontend UI Flow

```
Login (Google OAuth)
    │
    ▼
Upload CSV (drag & drop Google Takeout CSV)
    │
    ▼
Select Bookmarks (checkboxes, search/filter)
    │
    ▼
Connect uMap (URL + username + password)
    │
    ▼
Transfer (progress bar → result link)
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
    google_user: Option<GoogleUser>,     // name, email, avatar_url
    bookmarks: Option<Vec<GooglePlace>>, // parsed from CSV
    selected_ids: Option<Vec<usize>>,    // indices user checked
    umap_auth: Option<CookieAuth>,       // sessionid + csrftoken
}
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

## Test Commands

```bash
# Unit tests (core library)
cargo test -p core

# CLI (unchanged behavior)
cargo run -p cli -- --takeout examples/2026北海道_updated.csv \
  --umap-url "http://localhost:8000/en/" \
  --umap-cookie "sessionid=xxx; csrftoken=yyy" \
  --create-map "Test Map"

# Web server
cargo run -p web
# → http://localhost:3000

# uMap (local instance for testing)
podman compose -f umap/docker-compose.yml up -d
# → http://localhost:8000/en/
```

## Build Order

1. Create workspace `Cargo.toml` → migrate `src/` → `core/` + `cli/`
2. `core/src/umap/login.rs` — proxy login
3. `core/src/umap/upload.rs` — private map (`share_status=0`)
4. Verify CLI still works
5. `web/` server skeleton + embedded Svelte
6. Svelte frontend (all components)
7. API handlers
8. `web/build.rs` — wire Svelte build into Rust compile
9. End-to-end test with podman compose
