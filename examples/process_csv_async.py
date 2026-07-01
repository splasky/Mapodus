#!/usr/bin/env python3
"""Process CSV files to add GPS coordinates, place name, and star rating from Google Maps URLs - Async version."""

import csv
import re
import os
import asyncio
import aiohttp
from urllib.parse import unquote

# Concurrency settings
MAX_CONCURRENT_REQUESTS = 10
RATE_LIMIT_DELAY = 0.2  # seconds between batches

def extract_coords_from_search_url(url):
    """Extract coordinates from search URLs like /maps/search/24.8583332,120.9927297"""
    match = re.search(r'/maps/search/([-\d.]+),([-\d.]+)', url)
    if match:
        return match.group(1), match.group(2)
    return None, None

async def fetch_url(session, url, semaphore):
    """Fetch a URL with rate limiting."""
    async with semaphore:
        try:
            headers = {
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
                'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8',
                'Accept-Language': 'en-US,en;q=0.5',
            }
            async with session.get(url, headers=headers, timeout=aiohttp.ClientTimeout(total=30), allow_redirects=True) as response:
                content = await response.text()
                final_url = str(response.url)
                return content, final_url
        except Exception as e:
            print(f"    Error fetching: {e}")
            return None, None

def parse_google_maps_response(content, final_url):
    """Parse the response to extract coordinates, name, and rating."""
    if not content:
        return None, None, None, None

    lat, lng = None, None
    name, rating = None, None

    # Method 1: Check final URL for @lat,lng pattern
    coord_match = re.search(r'@([-\d.]+),([-\d.]+)', final_url)
    if coord_match:
        lat, lng = coord_match.group(1), coord_match.group(2)

    # Method 2: Look for coordinates in page content
    if not lat:
        patterns = [
            r'!3d([-\d.]+)!4d([-\d.]+)',
            r'\[null,null,([-\d.]+),([-\d.]+)\]',
            r'center":\[([-\d.]+),([-\d.]+)\]',
            r'"lat":([-\d.]+),"lng":([-\d.]+)',
            r'\\x22([-\d.]+)\\x22,\\x22([-\d.]+)\\x22',
        ]

        for pattern in patterns:
            matches = re.findall(pattern, content)
            for match in matches:
                try:
                    potential_lat = float(match[0])
                    potential_lng = float(match[1])
                    # Validate coordinates for Asia-Pacific region
                    if (20 < abs(potential_lat) < 50 and 100 < abs(potential_lng) < 150):
                        lat, lng = str(potential_lat), str(potential_lng)
                        break
                except (ValueError, IndexError):
                    continue
            if lat:
                break

    # Method 3: Look for coordinates in APP_OPTIONS or similar
    if not lat:
        coord_matches = re.findall(r'(2[0-6]\.\d{4,})[,\]"\s]+(1[12][0-9]\.\d{4,})', content)
        for match in coord_matches:
            try:
                potential_lat = float(match[0])
                potential_lng = float(match[1])
                if (20 < potential_lat < 50 and 100 < potential_lng < 150):
                    lat, lng = str(potential_lat), str(potential_lng)
                    break
            except:
                continue

    # Extract rating
    rating_patterns = [
        r',(\d\.\d),"[^"]*review',
        r'"rating":\s*(\d+\.?\d*)',
        r'(\d+\.?\d*)\s*(?:stars?|顆星)',
        r'"averageRating":(\d+\.?\d*)',
        r'aria-label="(\d+\.?\d*)',
    ]

    for pattern in rating_patterns:
        match = re.search(pattern, content, re.IGNORECASE)
        if match:
            try:
                potential_rating = float(match.group(1))
                if 1 <= potential_rating <= 5:
                    rating = str(potential_rating)
                    break
            except ValueError:
                continue

    # Extract place name from page title
    title_match = re.search(r'<title>([^<]+)</title>', content)
    if title_match:
        page_title = title_match.group(1)
        name = re.sub(r'\s*[-–·]\s*Google\s*Maps?.*$', '', page_title, flags=re.IGNORECASE).strip()
        if name in ('Google Maps', 'Google 地圖', ''):
            name = None

    return lat, lng, name, rating

async def process_url(session, semaphore, index, title, url):
    """Process a single URL and return results."""
    if not url or not url.startswith('http'):
        return index, None, None, None, None

    # Handle search URLs with coordinates directly
    if '/maps/search/' in url:
        lat, lng = extract_coords_from_search_url(url)
        return index, lat, lng, None, None

    print(f"  [{index}] Fetching: {title[:40] if title else url[:50]}...")
    content, final_url = await fetch_url(session, url, semaphore)
    lat, lng, name, rating = parse_google_maps_response(content, final_url)
    return index, lat, lng, name, rating

async def process_csv_file_async(input_path, output_path):
    """Process a single CSV file asynchronously."""
    print(f"\nProcessing: {input_path}")

    # Read all rows first
    rows_data = []
    with open(input_path, 'r', encoding='utf-8') as f:
        reader = csv.reader(f)
        headers = next(reader)
        new_headers = headers + ['緯度(Latitude)', '經度(Longitude)', '地點名稱(Place Name)', '星級評分(Rating)']

        for idx, row in enumerate(reader):
            rows_data.append((idx, row))

    # Prepare tasks
    semaphore = asyncio.Semaphore(MAX_CONCURRENT_REQUESTS)
    results = {}

    async with aiohttp.ClientSession() as session:
        tasks = []
        for idx, row in rows_data:
            url = row[2] if len(row) > 2 else ''
            title = row[0] if len(row) > 0 else ''
            tasks.append(process_url(session, semaphore, idx, title, url))

        # Process in batches to avoid overwhelming the server
        batch_size = MAX_CONCURRENT_REQUESTS * 2
        for i in range(0, len(tasks), batch_size):
            batch = tasks[i:i + batch_size]
            batch_results = await asyncio.gather(*batch)
            for result in batch_results:
                idx, lat, lng, name, rating = result
                results[idx] = (lat, lng, name, rating)
            await asyncio.sleep(RATE_LIMIT_DELAY)

    # Build output rows
    output_rows = [new_headers]
    for idx, row in rows_data:
        new_row = list(row)
        while len(new_row) < 5:
            new_row.append('')

        lat, lng, name, rating = results.get(idx, (None, None, None, None))
        new_row.extend([lat or '', lng or '', name or '', rating or ''])
        output_rows.append(new_row)

    # Write output
    with open(output_path, 'w', encoding='utf-8', newline='') as f:
        writer = csv.writer(f)
        writer.writerows(output_rows)

    print(f"  Saved to: {output_path}")

async def main():
    """Process all CSV files."""
    csv_files = [
        '大新竹.csv',
        '2025 Fukuoka.csv',
        '嘉義台南高雄地區.csv',
        '離島.csv',
        '大台北地區.csv',
        '台中.csv',
        '20251010台東.csv',
    ]

    base_dir = '/home/hychang/Downloads/takeout-20260123T111740Z-3-001/Takeout/已儲存'

    for csv_file in csv_files:
        input_path = os.path.join(base_dir, csv_file)
        output_path = os.path.join(base_dir, csv_file.replace('.csv', '_updated.csv'))

        if os.path.exists(input_path):
            await process_csv_file_async(input_path, output_path)
        else:
            print(f"File not found: {input_path}")

if __name__ == '__main__':
    asyncio.run(main())
