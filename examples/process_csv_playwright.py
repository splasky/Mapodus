#!/usr/bin/env python3
"""Process CSV files using Playwright for accurate GPS extraction from Google Maps."""

import csv
import re
import os
import asyncio
from playwright.async_api import async_playwright

# Concurrency settings
MAX_CONCURRENT_BROWSERS = 5
RATE_LIMIT_DELAY = 0.2

def extract_coords_from_search_url(url):
    """Extract coordinates from search URLs like /maps/search/24.8583332,120.9927297"""
    match = re.search(r'/maps/search/([-\d.]+),([-\d.]+)', url)
    if match:
        return match.group(1), match.group(2)
    return None, None

async def fetch_with_playwright(context, url, semaphore):
    """Fetch URL using Playwright and extract data."""
    async with semaphore:
        page = None
        coords_from_network = []

        async def capture_response(response):
            """Capture coordinates from network responses."""
            try:
                if response.status == 200 and 'maps' in response.url:
                    text = await response.text()
                    # Pattern for coordinates in Google Maps API responses
                    patterns = [
                        r'\[null,null,(2[0-6]\.\d{4,}),(1[12]\d\.\d{4,})\]',
                        r'"lat":(2[0-6]\.\d+).*?"lng":(1[12]\d\.\d+)',
                    ]
                    for pattern in patterns:
                        for m in re.findall(pattern, text):
                            try:
                                lat, lng = float(m[0]), float(m[1])
                                if 20 < lat < 50 and 100 < lng < 150:
                                    coords_from_network.append((lat, lng))
                            except:
                                pass
            except:
                pass

        try:
            page = await context.new_page()
            page.on('response', capture_response)

            # Navigate
            await page.goto(url, wait_until='domcontentloaded', timeout=30000)
            await asyncio.sleep(3)

            lat, lng = None, None
            name, rating = None, None

            # Method 1: Check URL for @lat,lng pattern
            final_url = page.url
            coord_match = re.search(r'@([-\d.]+),([-\d.]+)', final_url)
            if coord_match:
                lat, lng = coord_match.group(1), coord_match.group(2)

            # Method 2: Use coordinates from network responses
            if not lat and coords_from_network:
                # Use the first valid coordinate (usually the place location)
                lat, lng = str(coords_from_network[0][0]), str(coords_from_network[0][1])

            # Method 3: Extract from page content
            if not lat:
                content = await page.content()
                patterns = [
                    r'!3d(2[0-6]\.\d+)!4d(1[12]\d\.\d+)',
                    r'\[null,null,(2[0-6]\.\d+),(1[12]\d\.\d+)\]',
                ]
                for pattern in patterns:
                    matches = re.findall(pattern, content)
                    for m in matches:
                        try:
                            potential_lat = float(m[0])
                            potential_lng = float(m[1])
                            if 20 < potential_lat < 50 and 100 < potential_lng < 150:
                                lat, lng = str(potential_lat), str(potential_lng)
                                break
                        except:
                            pass
                    if lat:
                        break

            # Extract rating
            try:
                rating_element = await page.query_selector('[aria-label*="star"], [aria-label*="顆星"]')
                if rating_element:
                    aria_label = await rating_element.get_attribute('aria-label')
                    if aria_label:
                        rating_match = re.search(r'([\d.]+)', aria_label)
                        if rating_match:
                            potential_rating = float(rating_match.group(1))
                            if 1 <= potential_rating <= 5:
                                rating = str(potential_rating)
            except:
                pass

            # Fallback rating from content
            if not rating:
                try:
                    content = await page.content()
                    for pattern in [r',(\d\.\d),"[^"]*review', r'"rating":(\d+\.?\d*)']:
                        match = re.search(pattern, content)
                        if match:
                            potential_rating = float(match.group(1))
                            if 1 <= potential_rating <= 5:
                                rating = str(potential_rating)
                                break
                except:
                    pass

            # Extract place name
            try:
                title = await page.title()
                name = re.sub(r'\s*[-–·]\s*Google\s*(Maps?|地圖).*$', '', title, flags=re.IGNORECASE).strip()
                if name in ('Google Maps', 'Google 地圖', ''):
                    name = None
            except:
                pass

            return lat, lng, name, rating

        except Exception as e:
            return None, None, None, None
        finally:
            if page:
                await page.close()

async def process_url(context, semaphore, index, title, url):
    """Process a single URL and return results."""
    if not url or not url.startswith('http'):
        return index, None, None, None, None

    # Handle search URLs with coordinates directly
    if '/maps/search/' in url:
        lat, lng = extract_coords_from_search_url(url)
        print(f"  [{index}] {title[:30] if title else 'pin'} -> ({lat}, {lng}) [direct]")
        return index, lat, lng, None, None

    lat, lng, name, rating = await fetch_with_playwright(context, url, semaphore)

    status = []
    if lat and lng:
        status.append(f"{lat[:8]},{lng[:9]}")
    if rating:
        status.append(f"★{rating}")

    status_str = " | ".join(status) if status else "no data"
    print(f"  [{index}] {title[:35] if title else url[:35]}... -> {status_str}")

    return index, lat, lng, name, rating

async def process_csv_file_async(input_path, output_path, browser):
    """Process a single CSV file asynchronously."""
    print(f"\n{'='*60}")
    print(f"Processing: {os.path.basename(input_path)}")
    print(f"{'='*60}")

    # Read all rows first
    rows_data = []
    with open(input_path, 'r', encoding='utf-8') as f:
        reader = csv.reader(f)
        headers = next(reader)
        new_headers = headers + ['緯度(Latitude)', '經度(Longitude)', '地點名稱(Place Name)', '星級評分(Rating)']

        for idx, row in enumerate(reader):
            rows_data.append((idx, row))

    # Create browser context
    context = await browser.new_context(
        viewport={'width': 1280, 'height': 720},
        locale='zh-TW'
    )

    # Process with semaphore
    semaphore = asyncio.Semaphore(MAX_CONCURRENT_BROWSERS)
    results = {}
    total_with_coords = 0

    # Process in batches
    batch_size = MAX_CONCURRENT_BROWSERS * 2
    for i in range(0, len(rows_data), batch_size):
        batch = rows_data[i:i + batch_size]
        tasks = []
        for idx, row in batch:
            url = row[2] if len(row) > 2 else ''
            title = row[0] if len(row) > 0 else ''
            tasks.append(process_url(context, semaphore, idx + 1, title, url))

        batch_results = await asyncio.gather(*tasks)
        for result in batch_results:
            idx, lat, lng, name, rating = result
            results[idx - 1] = (lat, lng, name, rating)
            if lat and lng:
                total_with_coords += 1

        await asyncio.sleep(RATE_LIMIT_DELAY)

    await context.close()

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

    print(f"\n  ✓ Saved: {output_path}")
    print(f"  ✓ Coordinates: {total_with_coords}/{len(rows_data)} places")

async def main():
    """Process all CSV files."""
    csv_files = [
        '2025 Fukuoka.csv',
        '大台北地區.csv',
    ]

    base_dir = '/home/hychang/Downloads/takeout-20260123T111740Z-3-001/Takeout/已儲存'

    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)

        for csv_file in csv_files:
            input_path = os.path.join(base_dir, csv_file)
            output_path = os.path.join(base_dir, csv_file.replace('.csv', '_updated.csv'))

            if os.path.exists(input_path):
                await process_csv_file_async(input_path, output_path, browser)
            else:
                print(f"File not found: {input_path}")

        await browser.close()

if __name__ == '__main__':
    asyncio.run(main())
