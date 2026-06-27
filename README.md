# Google map marker to OSM map
> [!WARNING]
> This project is still under development. The code may change recently. Please don't use in production environments.

## Features
* Download user's Google Maps saved places (清單)
* Convert the data points onto uMap
* Upload data points onto uMap with fields: 標題(Title), 筆記(Notes), 網址(URL), 標籤(Tags), 留言(Comments), 緯度(Latitude), 經度(Longitude), 地點名稱(Place Name), 星級評分(Rating), 網站(Website), 簡介(Description), 原文名稱(Original Name), 英文名稱(English Name)

## Usage

```
google-maps-to-umap [OPTIONS]
```

### Options

| Flag | Description |
|------|-------------|
| `-t, --takeout <FILE>` | Path to Google Takeout CSV or JSON file |
| `-g, --geojson <FILE>` | Path to existing GeoJSON file (alternative to --takeout) |
| `-o, --output <FILE>` | Output GeoJSON file path (skip uMap upload) |
| `--umap-url <URL>` | uMap instance URL (default: `https://umap.openstreetmap.fr/en/`) |
| `--umap-map-id <ID>` | uMap map ID to upload to |
| `--umap-cookie <COOKIE>` | uMap session cookie (format: `sessionid=xxx; csrftoken=xxx`) |
| `--layer-name <NAME>` | Target layer name (default: `Google Maps Saved`) |
| `-h, --help` | Print help |

### Examples

**Convert Google Takeout export and upload to uMap:**
```
google-maps-to-umap \
  --takeout ./Saved.csv \
  --umap-map-id 123456 \
  --umap-cookie "sessionid=xxx; csrftoken=xxx"
```

**Convert only (save GeoJSON locally):**
```
google-maps-to-umap \
  --takeout ./Saved.csv \
  --output ./places.geojson
```

**Upload existing GeoJSON to uMap:**
```
google-maps-to-umap \
  --geojson ./places.geojson \
  --umap-map-id 123456 \
  --umap-cookie "sessionid=xxx; csrftoken=xxx"
```

## Authentication

### Google Maps Data
The tool reads saved places from **Google Takeout** exports. To get your data:
1. Go to [takeout.google.com](https://takeout.google.com)
2. Deselect all, then select **Saved** (export as CSV) and/or **Maps (your places)** (export as JSON/GeoJSON)
3. Download and extract the archive

### uMap Authentication
uMap does not yet have a stable public API with API keys. Authentication is done via **browser session cookies**:
1. Log in to [umap.openstreetmap.fr](https://umap.openstreetmap.fr/en/) in your browser
2. Open Developer Tools → Application/Storage → Cookies
3. Copy the `sessionid` and `csrftoken` values
4. Pass them as `--umap-cookie "sessionid=xxx; csrftoken=xxx"`

## Build

```
cargo build --release
```

