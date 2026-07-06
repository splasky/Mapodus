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
        field_name: &str,
    ) -> Option<usize> {
        let aliases = header_aliases(field_name);

        aliases
            .iter()
            .find_map(|alias| header_map.get(&normalize_header(alias)).copied())
            .or_else(|| {
                header_map
                    .iter()
                    .find(|(header, _)| {
                        aliases
                            .iter()
                            .any(|alias| header.contains(&normalize_header(alias)))
                    })
                    .map(|(_, &idx)| idx)
            })
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
            .map(|(i, header)| (normalize_header(header), i))
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

fn normalize_header(header: &str) -> String {
    header
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '(' | ')' | '（' | '）' | '[' | ']' | '【' | '】' | '-' | '_' | '/'
                )
        })
        .collect()
}

fn header_aliases(field_name: &str) -> &'static [&'static str] {
    match field_name {
        "Title" => &["title", "標題", "标题", "タイトル", "題名", "제목"],
        "Notes" => &["notes", "note", "筆記", "笔记", "メモ", "ノート", "메모"],
        "URL" => &["url", "網址", "网址", "リンク", "link"],
        "Tags" => &["tags", "tag", "標籤", "标签", "タグ", "ラベル", "태그"],
        "Comments" => &[
            "comments",
            "comment",
            "留言",
            "コメント",
            "コメント欄",
            "댓글",
        ],
        "Latitude" => &["latitude", "lat", "緯度", "纬度", "緯度latitude", "위도"],
        "Longitude" => &["longitude", "lng", "lon", "經度", "经度", "経度", "경도"],
        "Place Name" => &[
            "place name",
            "place",
            "地點名稱",
            "地点名称",
            "場所名",
            "場所の名前",
            "장소 이름",
        ],
        "Rating" => &[
            "rating",
            "星級評分",
            "评分",
            "評分",
            "評価",
            "レーティング",
            "평점",
        ],
        "Website" => &[
            "website",
            "web site",
            "網站",
            "网站",
            "ウェブサイト",
            "サイト",
        ],
        "Description" => &["description", "簡介", "简介", "説明", "설명"],
        "Original Name" => &[
            "original name",
            "原文名稱",
            "原文名称",
            "元の名前",
            "원래 이름",
        ],
        "English Name" => &[
            "english name",
            "英文名稱",
            "英文名称",
            "英語名",
            "영어 이름",
        ],
        _ => &[],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_japanese_takeout_headers() {
        let headers =
            csv::StringRecord::from(vec!["タイトル", "メモ", "リンク", "タグ", "コメント"]);
        let record = csv::StringRecord::from(vec![
            "東京タワー",
            "夜景",
            "https://www.google.com/maps/search/35.6586,139.7454",
            "旅行",
            "再訪",
        ]);

        let place = GooglePlace::from_csv_record(&record, &headers);

        assert_eq!(place.title.as_deref(), Some("東京タワー"));
        assert_eq!(place.notes.as_deref(), Some("夜景"));
        assert_eq!(
            place.url.as_deref(),
            Some("https://www.google.com/maps/search/35.6586,139.7454")
        );
        assert_eq!(place.tags.as_deref(), Some("旅行"));
        assert_eq!(place.comments.as_deref(), Some("再訪"));
        assert_eq!(place.latitude.as_deref(), Some("35.6586"));
        assert_eq!(place.longitude.as_deref(), Some("139.7454"));
    }

    #[test]
    fn parses_localized_detail_headers_with_english_hints() {
        let headers = csv::StringRecord::from(vec![
            "標題",
            "網址",
            "緯度(Latitude)",
            "經度(Longitude)",
            "場所名",
            "評価",
            "ウェブサイト",
            "説明",
            "元の名前",
            "英語名",
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
            "Tokyo Skytree",
        ]);

        let place = GooglePlace::from_csv_record(&record, &headers);

        assert_eq!(place.title.as_deref(), Some("スカイツリー"));
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
        assert_eq!(place.english_name.as_deref(), Some("Tokyo Skytree"));
        assert_eq!(place.place_id.as_deref(), Some("abc123"));
    }
}
