#!/usr/bin/env python3
"""Convert Google Maps Saved CSV files to GeoJSON format (uMap compatible)."""

import csv
import json
import os
import re
import time
import sys
import requests

BASE_DIR = os.path.dirname(os.path.abspath(__file__))

GEOCODER_USER_AGENT = 'GoogleMapsExportConverter/1.0 (maps-export-converter)'

NON_MAP_FILES = {'儲存的網頁.csv', '圖片.csv', '最愛的地點.csv', '想去的地點.csv'}

CSV_PAIRS = [
    ('台中.csv', '台中_updated.csv'),
    ('2025 Fukuoka.csv', '2025 Fukuoka_updated.csv'),
    ('嘉義台南高雄地區.csv', '嘉義台南高雄地區_updated.csv'),
    ('離島.csv', '離島_updated.csv'),
    ('大台北地區.csv', '大台北地區_updated.csv'),
    ('大新竹.csv', '大新竹_updated.csv'),
    ('20251010台東.csv', '20251010台東_updated.csv'),
    ('2026北海道.csv', None),
]


def is_valid_coord(lat_str, lng_str):
    if not lat_str or not lng_str:
        return False
    try:
        lat = float(lat_str)
        lng = float(lng_str)
        return -90 <= lat <= 90 and -180 <= lng <= 180 and abs(lat) > 0.01 and abs(lng) > 0.01
    except (ValueError, TypeError):
        return False


def is_placeholder_coord(lat_str, lng_str):
    """Check if coordinates are Google Maps default/placeholder."""
    if not lat_str or not lng_str:
        return False
    try:
        lat = float(lat_str)
        lng = float(lng_str)
        return abs(lat - 22.272) < 0.01 and abs(lng - 119.596) < 0.01
    except (ValueError, TypeError):
        return False


def extract_coords_from_url(url):
    """Extract coordinates from search URLs like /maps/search/24.8583332,120.9927297"""
    if not url:
        return None, None
    match = re.search(r'/maps/search/([-\d.]+),([-\d.]+)', url)
    if match:
        return match.group(1), match.group(2)
    return None, None


def geocode_place(name, retries=2):
    """Use Nominatim to find coordinates for a place."""
    if not name:
        return None, None
    headers = {
        'User-Agent': GEOCODER_USER_AGENT,
        'Accept': 'application/json',
    }
    for attempt in range(retries):
        try:
            resp = requests.get(
                'https://nominatim.openstreetmap.org/search',
                params={'q': name, 'format': 'json', 'limit': 1},
                headers=headers,
                timeout=10
            )
            if resp.status_code == 200:
                data = resp.json()
                if data:
                    return data[0]['lat'], data[0]['lon']
        except Exception:
            if attempt < retries - 1:
                time.sleep(1)
    return None, None


def fetch_coords_from_google_maps(url, retries=2):
    """Fetch Google Maps URL and try to extract coordinates."""
    if not url or not url.startswith('http'):
        return None, None, None, None
    lat, lng = extract_coords_from_url(url)
    if lat and lng:
        return lat, lng, None, None
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
        'Accept-Language': 'en-US,en;q=0.9,ja;q=0.8',
    }
    for attempt in range(retries):
        try:
            resp = requests.get(url, headers=headers, timeout=15, allow_redirects=True)
            final_url = resp.url
            content = resp.text

            lat, lng, name, rating = None, None, None, None

            # Method 1: @lat,lng from final URL
            coord_match = re.search(r'@([-\d.]+),([-\d.]+)', final_url)
            if coord_match:
                lat, lng = coord_match.group(1), coord_match.group(2)

            # Method 2: !3dXX!4dXX patterns
            if not lat:
                patterns = [
                    r'!3d([-\d.]+)!4d([-\d.]+)',
                    r'\[null,null,([-\d.]+),([-\d.]+)\]',
                    r'"lat":([-\d.]+),"lng":([-\d.]+)',
                ]
                for pattern in patterns:
                    matches = re.findall(pattern, content)
                    for m in matches:
                        try:
                            pl = float(m[0])
                            plng = float(m[1])
                            if -90 <= pl <= 90 and -180 <= plng <= 180 and abs(pl) > 0.01:
                                lat, lng = str(pl), str(plng)
                                break
                        except (ValueError, IndexError):
                            pass
                    if lat:
                        break

            if is_valid_coord(lat, lng):
                return lat, lng, name, rating
            if attempt < retries - 1:
                time.sleep(1)
        except Exception:
            if attempt < retries - 1:
                time.sleep(2)
    return None, None, None, None


def read_updated_csv(path):
    """Read _updated.csv and return a dict keyed by URL."""
    result = {}
    if not os.path.exists(path):
        return result
    with open(path, 'r', encoding='utf-8') as f:
        reader = csv.reader(f)
        headers = next(reader)
        for row in reader:
            if len(row) < 7:
                continue
            title = row[0].strip() if row[0] else ''
            url = row[2].strip() if len(row) > 2 and row[2] else ''
            notes = row[1].strip() if len(row) > 1 and row[1] else ''
            tags = row[3].strip() if len(row) > 3 and row[3] else ''
            lat = row[5].strip() if len(row) > 5 and row[5] else ''
            lng = row[6].strip() if len(row) > 6 and row[6] else ''
            name = row[7].strip() if len(row) > 7 and row[7] else ''
            rating = row[8].strip() if len(row) > 8 and row[8] else ''

            # Handle malformed rows where URL appears in wrong column
            if row[0] and row[0].startswith('http') and url.startswith('http'):
                name = ''
                rating = ''

            if title or url:
                result[url] = (lat, lng, name, rating, notes, tags)
    return result


def make_description(name, url, rating, note, tags):
    lines = [f"名稱: {name}"]
    if url:
        lines.append(f"Google Maps: {url}")
    lines.append(f"Rating: {rating or ''}")
    lines.append(f"Note: {note or ''}")
    lines.append(f"Tags: {tags or ''}")
    return '\n'.join(lines)


def build_feature(name, url, lng, lat, rating, note, tags):
    return {
        "type": "Feature",
        "properties": {
            "name": name,
            "description": make_description(name, url, rating, note, tags),
        },
        "geometry": {
            "type": "Point",
            "coordinates": [float(lng), float(lat)]
        }
    }


def process_csv(orig_name, updated_name):
    orig_path = os.path.join(BASE_DIR, orig_name)
    if not os.path.exists(orig_path):
        print(f"  ✗ Not found: {orig_name}")
        return

    updated_data = {}
    has_any_valid_coord_in_updated = False

    if updated_name:
        updated_data = read_updated_csv(os.path.join(BASE_DIR, updated_name))
        # Check if _updated has any valid coords (to know if we should bother fetching)
        for v in updated_data.values():
            if is_valid_coord(v[0], v[1]) and not is_placeholder_coord(v[0], v[1]):
                has_any_valid_coord_in_updated = True
                break

    features = []
    fetch_queue = []
    coords_existing = 0
    coords_geocoded = 0

    with open(orig_path, 'r', encoding='utf-8') as f:
        reader = csv.reader(f)
        headers = next(reader)
        for row in reader:
            if len(row) < 3:
                continue
            title = row[0].strip() if row[0] else ''
            notes = row[1].strip() if len(row) > 1 and row[1] else ''
            url = row[2].strip() if len(row) > 2 and row[2] else ''
            tags = row[3].strip() if len(row) > 3 and row[3] else ''

            if not title and not url:
                continue
            if not url or not url.startswith('http'):
                continue
            if 'google.com/maps' not in url:
                continue

            lat, lng, name, rating = None, None, None, None
            need_fetch = True

            if url in updated_data:
                ulat, ulng, uname, urating, unotes, utags = updated_data[url]
                if is_valid_coord(ulat, ulng) and not is_placeholder_coord(ulat, ulng):
                    lat, lng = ulat, ulng
                    name = uname or title
                    rating = urating
                    if unotes: notes = unotes
                    if utags: tags = utags
                    coords_existing += 1
                    need_fetch = False
                elif is_valid_coord(ulat, ulng) and is_placeholder_coord(ulat, ulng):
                    need_fetch = True
                else:
                    # Empty coords - try geocoding for known place names (Japan etc.)
                    need_fetch = True

            if need_fetch:
                fetch_queue.append((title, url, notes, tags))

            if lat and lng:
                features.append(build_feature(
                    name or title, url, lng, lat, rating or '', notes, tags
                ))

    # Geocode missing entries
    if fetch_queue:
        total = len(fetch_queue)
        print(f"  Geocoding {total} places...")
        for i, (title, url, notes, tags) in enumerate(fetch_queue, 1):
            sys.stdout.write(f"\r    [{i}/{total}] {title[:45]:45s}")
            sys.stdout.flush()

            # Try to fetch from Google Maps first
            lat, lng, gname, grating = fetch_coords_from_google_maps(url)

            # Fallback to Nominatim geocoding
            if not is_valid_coord(lat, lng):
                lat, lng = geocode_place(title)

            if is_valid_coord(lat, lng):
                features.append(build_feature(
                    title, url, lng, lat, '', notes, tags
                ))
                coords_geocoded += 1

            time.sleep(0.5)

    # Write GeoJSON
    geojson_name = orig_name.replace('.csv', '.geojson')
    geojson_path = os.path.join(BASE_DIR, geojson_name)
    fc = {"type": "FeatureCollection", "features": features}
    with open(geojson_path, 'w', encoding='utf-8') as f:
        json.dump(fc, f, ensure_ascii=False, indent=2)

    print(f"\n  ✓ {geojson_name} ({len(features)} features, {coords_existing} existing + {coords_geocoded} geocoded)")
    return len(features)


def main():
    print("=" * 60)
    print("CSV → GeoJSON Converter (uMap format)")
    print("=" * 60)

    total_features = 0
    for orig_name, updated_name in CSV_PAIRS:
        if orig_name in NON_MAP_FILES:
            print(f"\n  - Skipping: {orig_name} (non-map data)")
            continue
        print(f"\nProcessing: {orig_name}")
        count = process_csv(orig_name, updated_name)
        total_features += count

    print(f"\n{'=' * 60}")
    print(f"Done! Generated GeoJSON for {total_features} features total.")
    print(f"{'=' * 60}")


if __name__ == '__main__':
    main()
