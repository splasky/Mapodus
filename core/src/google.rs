use anyhow::{Result, anyhow};
use csv;
use serde::{Deserialize, Serialize};
use serde_json;

/// Extract coordinates from a Google Maps URL.
/// Handles formats:
///   - `https://maps.google.com/?q=lat,lng`
///   - `https://www.google.com/maps/place/Name/@lat,lng,zoom`
///   - `https://www.google.com/maps/place/?q=place_id:XYZ` (no coords)
pub fn extract_coords_from_url(url: &str) -> Option<(f64, f64)> {
    // Pattern: @lat,lng or @lat,lng,zoom in the path
    if let Some(at_pos) = url.find('@') {
        let after_at = &url[at_pos + 1..];
        let end = after_at
            .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != ',')
            .unwrap_or(after_at.len());
        let coords = &after_at[..end];
        let parts: Vec<&str> = coords.split(',').collect();
        if parts.len() >= 2 {
            let lat = parts[0].parse::<f64>().ok()?;
            let lng = parts[1].parse::<f64>().ok()?;
            if lat.is_finite() && lng.is_finite() {
                return Some((lat, lng));
            }
        }
    }

    // Pattern: ?q=lat,lng in query string
    if let Some(q_pos) = url.find("?q=") {
        let after_q = &url[q_pos + 3..];
        let end = after_q
            .find(|c: char| ['&', '#'].contains(&c))
            .unwrap_or(after_q.len());
        let value = &after_q[..end];
        // Only parse if it doesn't look like place_id: prefix
        if !value.starts_with("place_id:") {
            let parts: Vec<&str> = value.split(',').collect();
            if parts.len() >= 2 {
                let lat = parts[0].parse::<f64>().ok()?;
                let lng = parts[1].parse::<f64>().ok()?;
                if lat.is_finite() && lng.is_finite() {
                    return Some((lat, lng));
                }
            }
        }
    }

    None
}

/// Extract a Google Maps place ID from a URL.
/// Format: `https://www.google.com/maps/place/?q=place_id:ChIJ...`
pub fn extract_place_id_from_url(url: &str) -> Option<String> {
    // Pattern: place_id:XYZ in query string
    if let Some(q_pos) = url.find("place_id:") {
        let after = &url[q_pos + 9..];
        let end = after
            .find(|c: char| ['&', '#'].contains(&c))
            .unwrap_or(after.len());
        let id = &after[..end];
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GooglePlace {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub url: Option<String>,
    pub tags: Option<String>,
    pub comments: Option<String>,
    pub latitude: Option<String>,
    pub longitude: Option<String>,
    pub place_name: Option<String>,
    pub rating: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
    pub original_name: Option<String>,
    pub english_name: Option<String>,
    pub place_id: Option<String>,
}

impl GooglePlace {
    fn find_header_index(
        header_map: &std::collections::HashMap<String, usize>,
        english_name: &str,
    ) -> Option<usize> {
        let lower = english_name.to_lowercase();

        if let Some(&idx) = header_map.get(&lower) {
            return Some(idx);
        }

        if let Some((_, &idx)) = header_map.iter().find(|(key, _)| key.contains(&lower)) {
            return Some(idx);
        }

        let chinese_map: &[(&str, &str)] = &[
            ("標題", "title"),
            ("筆記", "notes"),
            ("網址", "url"),
            ("標籤", "tags"),
            ("留言", "comments"),
            ("緯度", "latitude"),
            ("經度", "longitude"),
            ("地點名稱", "place name"),
            ("星級評分", "rating"),
            ("網站", "website"),
            ("簡介", "description"),
            ("原文名稱", "original name"),
            ("英文名稱", "english name"),
        ];

        for (cn, en) in chinese_map {
            if (*en == lower || *en == english_name.to_lowercase())
                && let Some(&idx) = header_map.get(&cn.to_string())
            {
                return Some(idx);
            }
        }

        None
    }

    pub fn from_csv_record(record: &csv::StringRecord, headers: &csv::StringRecord) -> Self {
        let mut place = GooglePlace {
            title: None,
            notes: None,
            url: None,
            tags: None,
            comments: None,
            latitude: None,
            longitude: None,
            place_name: None,
            rating: None,
            website: None,
            description: None,
            original_name: None,
            english_name: None,
            place_id: None,
        };

        let header_map = headers
            .iter()
            .enumerate()
            .map(|(i, header)| (header.to_lowercase(), i))
            .collect::<std::collections::HashMap<String, usize>>();

        macro_rules! set_field {
            ($field:ident, $header:expr) => {
                if let Some(idx) = Self::find_header_index(&header_map, $header) {
                    place.$field = record.get(idx).map(|s| s.to_string());
                }
            };
        }

        set_field!(title, "Title");
        set_field!(notes, "Notes");
        set_field!(url, "URL");
        set_field!(tags, "Tags");
        set_field!(comments, "Comments");
        set_field!(latitude, "Latitude");
        set_field!(longitude, "Longitude");
        set_field!(place_name, "Place Name");
        set_field!(rating, "Rating");
        set_field!(website, "Website");
        set_field!(description, "Description");
        set_field!(original_name, "Original Name");
        set_field!(english_name, "English Name");

        // If lat/lng are missing, try to extract from URL
        if (place.latitude.is_none() || place.longitude.is_none())
            && let Some(url) = &place.url
            && let Some((lat, lng)) = extract_coords_from_url(url)
        {
            place.latitude = Some(lat.to_string());
            place.longitude = Some(lng.to_string());
        }
        // Extract place_id from URL if available
        if place.place_id.is_none()
            && let Some(url) = &place.url
            && let Some(pid) = extract_place_id_from_url(url)
        {
            place.place_id = Some(pid);
        }

        place
    }

    pub fn from_geojson_feature(feature: &serde_json::Value) -> Self {
        let empty_map = serde_json::Map::new();
        let properties = feature
            .get("properties")
            .and_then(|v| v.as_object())
            .unwrap_or(&empty_map);

        GooglePlace {
            title: properties
                .get("Title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            notes: properties
                .get("Notes")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            url: properties
                .get("URL")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            tags: properties
                .get("Tags")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            comments: properties
                .get("Comments")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            latitude: properties
                .get("Latitude")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            longitude: properties
                .get("Longitude")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            place_name: properties
                .get("Place Name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            rating: properties
                .get("Rating")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            website: properties
                .get("Website")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            description: properties
                .get("Description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            original_name: properties
                .get("Original Name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            english_name: properties
                .get("English Name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            place_id: None,
        }
    }
}

pub fn parse_takeout(path: &str) -> Result<Vec<GooglePlace>> {
    let file = std::fs::File::open(path)?;
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "csv" => {
            let mut reader = csv::Reader::from_reader(file);
            let headers = reader.headers()?.clone();
            let mut places = Vec::new();

            for result in reader.records() {
                let record = result?;
                if record.iter().all(|f| f.is_empty()) {
                    continue;
                }
                places.push(GooglePlace::from_csv_record(&record, &headers));
            }

            Ok(places)
        }
        "json" | "geojson" => {
            let json_value: serde_json::Value = serde_json::from_reader(file)?;
            let features = json_value
                .get("features")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow!("Invalid GeoJSON format"))?;

            let mut places = Vec::new();
            for feature in features {
                places.push(GooglePlace::from_geojson_feature(feature));
            }

            Ok(places)
        }
        _ => Err(anyhow!(
            "Unsupported file format. Expected .csv, .json, or .geojson"
        )),
    }
}
