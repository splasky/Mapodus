use anyhow::{Result, anyhow};
use csv;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportLocale {
    Auto,
    En,
    ZhTw,
}

#[derive(Debug, Clone, Copy)]
enum CsvField {
    Title,
    Notes,
    Url,
    Tags,
    Comments,
    Latitude,
    Longitude,
    PlaceName,
    Rating,
    Website,
    Description,
    OriginalName,
}

impl CsvField {
    fn canonical_headers(self) -> &'static [&'static str] {
        match self {
            CsvField::Title => &["title"],
            CsvField::Notes => &["notes", "note"],
            CsvField::Url => &["url"],
            CsvField::Tags => &["tags", "tag"],
            CsvField::Comments => &["comments", "comment"],
            CsvField::Latitude => &["latitude", "lat"],
            CsvField::Longitude => &["longitude", "lng", "lon"],
            CsvField::PlaceName => &["place name", "place"],
            CsvField::Rating => &["rating"],
            CsvField::Website => &["website", "web site"],
            CsvField::Description => &["description"],
            CsvField::OriginalName => &["original name"],
        }
    }

    fn zh_tw_headers(self) -> &'static [&'static str] {
        match self {
            CsvField::Title => &["標題"],
            CsvField::Notes => &["筆記"],
            CsvField::Url => &["網址"],
            CsvField::Tags => &["標籤"],
            CsvField::Comments => &["留言"],
            CsvField::Latitude => &["緯度"],
            CsvField::Longitude => &["經度"],
            CsvField::PlaceName => &["地點名稱"],
            CsvField::Rating => &["星級評分"],
            CsvField::Website => &["網站"],
            CsvField::Description => &["簡介"],
            CsvField::OriginalName => &["原文名稱"],
        }
    }
}

/// Extract coordinates from a Google Maps URL.
/// Handles formats:
///   - `https://maps.google.com/?q=lat,lng`
///   - `https://www.google.com/maps/place/Name/@lat,lng,zoom`
///   - `https://www.google.com/maps/place/?q=place_id:XYZ` (no coords)
///   - `https://www.google.com/maps/search/lat,lng`
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

    // Pattern: /search/lat,lng in path
    if let Some(search_pos) = url.find("/search/") {
        let after = &url[search_pos + 8..];
        let end = after
            .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != ',')
            .unwrap_or(after.len());
        let value = &after[..end];
        let parts: Vec<&str> = value.split(',').collect();
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

    // Pattern: /maps/search/lat,lng in the path
    if let Some(search_pos) = url.find("/maps/search/") {
        let after_search = &url[search_pos + "/maps/search/".len()..];
        let end = after_search
            .find(|c: char| ['?', '&', '#', '/'].contains(&c))
            .unwrap_or(after_search.len());
        let value = &after_search[..end];
        let parts: Vec<&str> = value.split(',').collect();
        if parts.len() >= 2 {
            let lat = parts[0].parse::<f64>().ok()?;
            let lng = parts[1].parse::<f64>().ok()?;
            if lat.is_finite() && lng.is_finite() {
                return Some((lat, lng));
            }
        }
    }

    None
}

/// Extract a Google Maps place ID from a URL.
/// Handles formats:
///   - `?q=place_id:ChIJ...`
///   - `/data=!4m2!3m1!1sChIJ...` (Google Takeout protobuf format)
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

    // Pattern: !1sPLACE_ID in protobuf-encoded data parameter
    // e.g. /data=!4m2!3m1!1s0x346835e9aa147b0b:0x8e09cb932ab96f34
    if let Some(s_pos) = url.find("!1s") {
        let after = &url[s_pos + 3..];
        let end = after
            .find(|c: char| ['!', '&', '#'].contains(&c))
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
    // This is the shared bookmark shape used by Takeout import, live Google
    // import, conversion, and upload. Fields stay optional because each source
    // exposes a different subset of place metadata.
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
    pub place_id: Option<String>,
    pub google_place_details: Option<serde_json::Value>,
}

impl GooglePlace {
    fn find_header_index(
        headers: &[NormalizedHeader],
        field: CsvField,
        locale: ImportLocale,
    ) -> Option<usize> {
        // Takeout localizes CSV headers. Try canonical aliases first, then
        // localized aliases when auto-detection or Traditional Chinese is used.
        find_header_by_aliases(headers, field.canonical_headers())
            .or_else(|| find_header_by_english_hint(headers, field.canonical_headers()))
            .or_else(|| {
                if matches!(locale, ImportLocale::Auto | ImportLocale::ZhTw) {
                    find_header_by_aliases(headers, field.zh_tw_headers())
                } else {
                    None
                }
            })
    }

    pub fn from_csv_record(record: &csv::StringRecord, headers: &csv::StringRecord) -> Self {
        Self::from_csv_record_with_locale(record, headers, ImportLocale::Auto)
    }

    pub fn from_csv_record_with_locale(
        record: &csv::StringRecord,
        headers: &csv::StringRecord,
        locale: ImportLocale,
    ) -> Self {
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
            place_id: None,
            google_place_details: None,
        };

        // Normalize once so field extraction can tolerate casing, whitespace,
        // and localized Takeout headers without duplicating lookup logic.
        let normalized_headers = headers
            .iter()
            .enumerate()
            .map(|(index, header)| NormalizedHeader::new(index, header))
            .collect::<Vec<_>>();

        macro_rules! set_field {
            ($field:ident, $csv_field:expr) => {
                if let Some(idx) = Self::find_header_index(&normalized_headers, $csv_field, locale)
                {
                    place.$field = record_value(record, idx);
                }
            };
        }

        set_field!(title, CsvField::Title);
        set_field!(notes, CsvField::Notes);
        set_field!(url, CsvField::Url);
        set_field!(tags, CsvField::Tags);
        set_field!(comments, CsvField::Comments);
        set_field!(latitude, CsvField::Latitude);
        set_field!(longitude, CsvField::Longitude);
        set_field!(place_name, CsvField::PlaceName);
        set_field!(rating, CsvField::Rating);
        set_field!(website, CsvField::Website);
        set_field!(description, CsvField::Description);
        set_field!(original_name, CsvField::OriginalName);

        apply_default_takeout_position_fallback(record, &mut place);
        apply_url_heuristic_fallback(record, &mut place);

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
        // Takeout GeoJSON stores bookmark metadata under title-cased property
        // names. Keep this parser strict so malformed files fail later during
        // coordinate conversion instead of inventing partial fields here.
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
            place_id: None,
            google_place_details: properties.get("google_place_details").cloned(),
        }
    }
}

#[derive(Debug, Clone)]
struct NormalizedHeader {
    index: usize,
    normalized: String,
    english_hints: Vec<String>,
}

impl NormalizedHeader {
    fn new(index: usize, header: &str) -> Self {
        Self {
            index,
            normalized: normalize_header(header),
            english_hints: english_hints(header),
        }
    }
}

fn normalize_header(header: &str) -> String {
    header
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .collect()
}

fn english_hints(header: &str) -> Vec<String> {
    let mut hints = Vec::new();
    let mut start = None;

    for (index, ch) in header.char_indices() {
        match ch {
            '(' | '（' | '[' | '【' => start = Some(index + ch.len_utf8()),
            ')' | '）' | ']' | '】' => {
                if let Some(start_index) = start.take() {
                    let hint = &header[start_index..index];
                    if hint.is_ascii() {
                        let normalized = normalize_header(hint);
                        if !normalized.is_empty() {
                            hints.push(normalized);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    hints
}

fn find_header_by_aliases(headers: &[NormalizedHeader], aliases: &[&str]) -> Option<usize> {
    aliases
        .iter()
        .map(|alias| normalize_header(alias))
        .find_map(|alias| {
            headers
                .iter()
                .find(|header| header.normalized == alias)
                .map(|header| header.index)
        })
}

fn find_header_by_english_hint(headers: &[NormalizedHeader], aliases: &[&str]) -> Option<usize> {
    aliases
        .iter()
        .map(|alias| normalize_header(alias))
        .find_map(|alias| {
            headers
                .iter()
                .find(|header| header.english_hints.iter().any(|hint| hint == &alias))
                .map(|header| header.index)
        })
}

fn record_value(record: &csv::StringRecord, index: usize) -> Option<String> {
    record
        .get(index)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn apply_default_takeout_position_fallback(record: &csv::StringRecord, place: &mut GooglePlace) {
    if record.len() < 5 {
        return;
    }

    if place.title.is_some()
        || place.notes.is_some()
        || place.url.is_some()
        || place.tags.is_some()
        || place.comments.is_some()
    {
        return;
    }

    if place.title.is_none() {
        place.title = record_value(record, 0);
    }
    if place.notes.is_none() {
        place.notes = record_value(record, 1);
    }
    if place.url.is_none() {
        place.url = record_value(record, 2);
    }
    if place.tags.is_none() {
        place.tags = record_value(record, 3);
    }
    if place.comments.is_none() {
        place.comments = record_value(record, 4);
    }
}

fn apply_url_heuristic_fallback(record: &csv::StringRecord, place: &mut GooglePlace) {
    if place.url.is_some() {
        return;
    }

    place.url = record
        .iter()
        .map(str::trim)
        .find(|value| looks_like_google_maps_url(value))
        .map(ToOwned::to_owned);
}

fn looks_like_google_maps_url(value: &str) -> bool {
    value.starts_with("https://www.google.com/maps/")
        || value.starts_with("https://maps.google.com/")
        || value.starts_with("https://goo.gl/maps/")
        || value.starts_with("https://maps.app.goo.gl/")
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
                // Takeout CSVs can contain spacer rows after manual edits.
                // Skipping them keeps row numbers stable without creating empty
                // features that conversion would drop later.
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_place_id_from_url tests ──

    #[test]
    fn test_extract_place_id_from_url_place_id_query() {
        let url = "https://www.google.com/maps/place/?q=place_id:ChIJN1t_tDeuEmsRUsoyG83frY4";
        assert_eq!(
            extract_place_id_from_url(url),
            Some("ChIJN1t_tDeuEmsRUsoyG83frY4".to_string())
        );
    }

    #[test]
    fn test_extract_place_id_from_url_1s_fmt() {
        let url = "https://www.google.com/maps/place/6owl+door+Hsinchu+Dongnan+Branch/data=!4m2!3m1!1s0x346835e9aa147b0b:0x8e09cb932ab96f34";
        assert_eq!(
            extract_place_id_from_url(url),
            Some("0x346835e9aa147b0b:0x8e09cb932ab96f34".to_string())
        );
    }

    #[test]
    fn test_extract_place_id_from_url_no_place_id() {
        let url = "https://www.google.com/maps/search/24.8583332,120.9927297";
        assert_eq!(extract_place_id_from_url(url), None);
    }

    #[test]
    fn test_extract_place_id_from_url_empty() {
        assert_eq!(extract_place_id_from_url(""), None);
    }

    // ── extract_coords_from_url tests ──

    #[test]
    fn test_extract_coords_from_url_at_fmt() {
        let url = "https://www.google.com/maps/place/Name/@24.7994433,120.9730098,17z";
        assert_eq!(
            extract_coords_from_url(url),
            Some((24.7994433, 120.9730098))
        );
    }

    #[test]
    fn test_extract_coords_from_url_search_fmt() {
        let url = "https://www.google.com/maps/search/24.8583332,120.9927297";
        assert_eq!(
            extract_coords_from_url(url),
            Some((24.8583332, 120.9927297))
        );
    }

    #[test]
    fn test_extract_coords_from_url_q_fmt() {
        let url = "https://maps.google.com/?q=25.033,121.565";
        assert_eq!(extract_coords_from_url(url), Some((25.033, 121.565)));
    }

    #[test]
    fn test_extract_coords_from_url_place_id_ignored() {
        let url = "https://www.google.com/maps/place/?q=place_id:ChIJN1t_tDeuEmsRUsoyG83frY4";
        assert_eq!(extract_coords_from_url(url), None);
    }

    #[test]
    fn test_extract_coords_from_url_no_coords() {
        let url = "https://www.google.com/maps/place/SomePlace/data=!4m2!3m1!1sabc123";
        assert_eq!(extract_coords_from_url(url), None);
    }

    #[test]
    fn test_extract_coords_from_url_empty() {
        assert_eq!(extract_coords_from_url(""), None);
    }

    // ── from_csv_record enrichment tests ──

    /// Parse a single CSV record with given headers and values, returning the GooglePlace.
    fn parse_one(headers: &[&str], values: &[&str]) -> GooglePlace {
        let mut wtr = csv::Writer::from_writer(Vec::new());
        wtr.write_record(headers).unwrap();
        wtr.write_record(values).unwrap();
        let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();

        let mut reader = csv::Reader::from_reader(data.as_bytes());
        let h = reader.headers().expect("headers").clone();
        let r = reader
            .records()
            .next()
            .expect("first record")
            .expect("valid record");
        GooglePlace::from_csv_record(&r, &h)
    }

    #[test]
    fn test_from_csv_record_minimal_extracts_place_id() {
        let place = parse_one(
            &["標題", "網址"],
            &[
                "Some Place",
                "https://www.google.com/maps/place/Some+Place/data=!4m2!3m1!1sChIJabc123",
            ],
        );
        assert_eq!(place.title.as_deref(), Some("Some Place"));
        assert_eq!(place.place_id.as_deref(), Some("ChIJabc123"));
        // No coords in this URL format
        assert!(place.latitude.is_none());
        assert!(place.longitude.is_none());
        // All other enrichment fields remain None
        assert!(place.place_name.is_none());
        assert!(place.rating.is_none());
        assert!(place.website.is_none());
        assert!(place.description.is_none());
        assert!(place.original_name.is_none());
    }

    #[test]
    fn test_from_csv_record_search_url_extracts_coords() {
        let place = parse_one(
            &["標題", "網址"],
            &[
                "",
                "https://www.google.com/maps/search/24.8583332,120.9927297",
            ],
        );
        assert_eq!(place.latitude.as_deref(), Some("24.8583332"));
        assert_eq!(place.longitude.as_deref(), Some("120.9927297"));
        assert!(place.place_id.is_none());
        assert!(place.title.is_none());
    }

    #[test]
    fn test_from_csv_record_no_url_no_enrichment() {
        let place = parse_one(&["標題", "筆記"], &["Place Name", "Some notes"]);
        assert_eq!(place.title.as_deref(), Some("Place Name"));
        assert_eq!(place.notes.as_deref(), Some("Some notes"));
        assert!(place.url.is_none());
        assert!(place.latitude.is_none());
        assert!(place.longitude.is_none());
        assert!(place.place_id.is_none());
    }

    #[test]
    fn test_from_csv_record_takeout_format_keeps_missing_fields_as_none() {
        // Realistic Takeout CSV: only 標題, 筆記, 網址, 標籤, 留言 — no extra fields
        let place = parse_one(
            &["標題", "筆記", "網址", "標籤", "留言"],
            &[
                "Some Place",
                "",
                "https://www.google.com/maps/place/Some+Place/data=!4m2!3m1!1sChIJabc123",
                "",
                "",
            ],
        );
        assert_eq!(place.title.as_deref(), Some("Some Place"));
        assert_eq!(place.place_id.as_deref(), Some("ChIJabc123"));
        assert!(place.rating.is_none());
        assert!(place.website.is_none());
        assert!(place.place_name.is_none());
        assert!(place.description.is_none());
        assert!(place.original_name.is_none());
        // Empty CSV fields are normalized away.
        assert!(place.notes.is_none());
        assert!(place.tags.is_none());
        assert!(place.comments.is_none());
    }

    #[test]
    fn test_from_csv_record_at_url_extracts_coords() {
        let place = parse_one(
            &["標題", "網址"],
            &[
                "Name",
                "https://www.google.com/maps/place/Name/@25.033,121.565,15z",
            ],
        );
        assert_eq!(place.latitude.as_deref(), Some("25.033"));
        assert_eq!(place.longitude.as_deref(), Some("121.565"));
    }

    #[test]
    fn test_from_csv_record_q_url_extracts_coords() {
        let place = parse_one(
            &["標題", "網址"],
            &["Name", "https://maps.google.com/?q=25.033,121.565"],
        );
        assert_eq!(place.latitude.as_deref(), Some("25.033"));
        assert_eq!(place.longitude.as_deref(), Some("121.565"));
    }
    #[test]
    fn extracts_coords_from_url_with_at_format() {
        let coords =
            extract_coords_from_url("https://www.google.com/maps/place/Test/@25.1972,55.2744,15z");
        assert_eq!(coords, Some((25.1972, 55.2744)));
    }

    #[test]
    fn extracts_coords_from_url_with_search() {
        let coords = extract_coords_from_url("https://www.google.com/maps/search/25.1972,55.2744");
        assert_eq!(coords, Some((25.1972, 55.2744)));
    }

    #[test]
    fn extracts_coords_from_url_with_q_params() {
        let coords = extract_coords_from_url("https://maps.google.com/?q=25.1972,55.2744");
        assert_eq!(coords, Some((25.1972, 55.2744)));
    }

    #[test]
    fn extracts_coords_skips_place_id_in_q_param() {
        let coords =
            extract_coords_from_url("https://www.google.com/maps/place/?q=place_id:ChIJabc123");
        assert_eq!(coords, None);
    }

    #[test]
    fn extracts_coords_returns_none_for_url_without_coords() {
        let coords = extract_coords_from_url("https://example.com");
        assert_eq!(coords, None);
    }

    #[test]
    fn extracts_coords_returns_none_for_empty_url() {
        let coords = extract_coords_from_url("");
        assert_eq!(coords, None);
    }

    #[test]
    fn extracts_place_id_from_standard_url() {
        let id = extract_place_id_from_url(
            "https://www.google.com/maps/place/?q=place_id:ChIJSANITIZED",
        );
        assert_eq!(id.as_deref(), Some("ChIJSANITIZED"));
    }

    #[test]
    fn extracts_place_id_returns_none_if_not_present() {
        let id = extract_place_id_from_url("https://www.google.com/maps/place/Test/@25.1,55.2");
        assert_eq!(id, None);
    }

    #[test]
    fn looks_like_google_maps_url_matches_variants() {
        assert!(looks_like_google_maps_url(
            "https://www.google.com/maps/place/Test"
        ));
        assert!(looks_like_google_maps_url("https://maps.google.com/maps"));
        assert!(looks_like_google_maps_url("https://goo.gl/maps/abc"));
        assert!(looks_like_google_maps_url("https://maps.app.goo.gl/abc"));
        assert!(!looks_like_google_maps_url("https://example.com"));
        assert!(!looks_like_google_maps_url(""));
    }

    #[test]
    fn normalize_header_removes_punctuation_and_whitespace() {
        assert_eq!(normalize_header("  Title  "), "title");
        assert_eq!(normalize_header("Place Name"), "placename");
        assert_eq!(normalize_header("星級評分(Rating)"), "星級評分rating");
        assert_eq!(normalize_header(""), "");
    }

    #[test]
    fn parses_unknown_locale_takeout_headers_by_default_position() {
        let headers = csv::StringRecord::from(vec![
            "العنوان",
            "ملاحظات",
            "الرابط",
            "التصنيفات",
            "التعليقات",
        ]);
        let record = csv::StringRecord::from(vec![
            "برج خليفة",
            "زيارة",
            "https://www.google.com/maps/search/25.1972,55.2744",
            "رحلة",
            "مساء",
        ]);

        let place = GooglePlace::from_csv_record(&record, &headers);

        assert_eq!(place.title.as_deref(), Some("برج خليفة"));
        assert_eq!(place.notes.as_deref(), Some("زيارة"));
        assert_eq!(
            place.url.as_deref(),
            Some("https://www.google.com/maps/search/25.1972,55.2744")
        );
        assert_eq!(place.tags.as_deref(), Some("رحلة"));
        assert_eq!(place.comments.as_deref(), Some("مساء"));
        assert_eq!(place.latitude.as_deref(), Some("25.1972"));
        assert_eq!(place.longitude.as_deref(), Some("55.2744"));
    }

    #[test]
    fn parses_localized_detail_headers_with_english_hints() {
        let headers = csv::StringRecord::from(vec![
            "標題",
            "網址",
            "緯度(Latitude)",
            "經度(Longitude)",
            "地點名稱(Place Name)",
            "星級評分(Rating)",
            "網站(Website)",
            "簡介(Description)",
            "原文名稱(Original Name)",
        ]);
        let record = csv::StringRecord::from(vec![
            "スカイツリー",
            "https://www.google.com/maps/place/?q=place_id:abc123",
            "35.7100",
            "139.8107",
            "東京スカイツリー",
            "4.5",
            "https://www.tokyo-skytree.jp/",
            "展望台",
            "東京スカイツリー",
        ]);

        let place = GooglePlace::from_csv_record(&record, &headers);

        assert_eq!(place.title.as_deref(), Some("スカイツリー"));
        assert_eq!(place.notes, None);
        assert_eq!(place.tags, None);
        assert_eq!(place.comments, None);
        assert_eq!(place.latitude.as_deref(), Some("35.7100"));
        assert_eq!(place.longitude.as_deref(), Some("139.8107"));
        assert_eq!(place.place_name.as_deref(), Some("東京スカイツリー"));
        assert_eq!(place.rating.as_deref(), Some("4.5"));
        assert_eq!(
            place.website.as_deref(),
            Some("https://www.tokyo-skytree.jp/")
        );
        assert_eq!(place.description.as_deref(), Some("展望台"));
        assert_eq!(place.original_name.as_deref(), Some("東京スカイツリー"));
        assert_eq!(place.place_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn finds_google_maps_url_without_recognized_url_header() {
        let headers = csv::StringRecord::from(vec!["unknown", "also unknown", "still unknown"]);
        let record = csv::StringRecord::from(vec![
            "not a url",
            "https://maps.app.goo.gl/example",
            "ignored",
        ]);

        let place = GooglePlace::from_csv_record(&record, &headers);

        assert_eq!(
            place.url.as_deref(),
            Some("https://maps.app.goo.gl/example")
        );
    }
}
