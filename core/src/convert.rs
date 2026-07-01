use crate::google::GooglePlace;
use geojson::{Feature, FeatureCollection, Geometry, Value};
use serde_json::Map;

pub struct Converter;

impl Converter {
    pub fn to_geojson(places: &[GooglePlace]) -> FeatureCollection {
        let features: Vec<Feature> = places
            .iter()
            .filter_map(|place| {
                let lat = place.latitude.as_ref().and_then(|s| s.parse::<f64>().ok());
                let lon = place.longitude.as_ref().and_then(|s| s.parse::<f64>().ok());

                if lat.is_none() && lon.is_none() {
                    return None;
                }

                let geometry = if let (Some(latitude), Some(longitude)) = (lat, lon) {
                    Geometry::new(Value::Point(vec![longitude, latitude]))
                } else {
                    return None;
                };

                let mut properties = Map::new();

                if let Some(title) = &place.title {
                    properties.insert(
                        "title".to_string(),
                        serde_json::Value::String(title.clone()),
                    );
                    properties.insert("標題".to_string(), serde_json::Value::String(title.clone()));
                }

                if let Some(notes) = &place.notes {
                    properties.insert(
                        "notes".to_string(),
                        serde_json::Value::String(notes.clone()),
                    );
                    properties.insert("筆記".to_string(), serde_json::Value::String(notes.clone()));
                }

                if let Some(url) = &place.url {
                    properties.insert("url".to_string(), serde_json::Value::String(url.clone()));
                    properties.insert("網址".to_string(), serde_json::Value::String(url.clone()));
                }

                if let Some(tags) = &place.tags {
                    properties.insert("tags".to_string(), serde_json::Value::String(tags.clone()));
                    properties.insert("標籤".to_string(), serde_json::Value::String(tags.clone()));
                }

                if let Some(comments) = &place.comments {
                    properties.insert(
                        "comments".to_string(),
                        serde_json::Value::String(comments.clone()),
                    );
                    properties.insert(
                        "留言".to_string(),
                        serde_json::Value::String(comments.clone()),
                    );
                }

                if let Some(lat_str) = &place.latitude {
                    properties.insert(
                        "latitude".to_string(),
                        serde_json::Value::String(lat_str.clone()),
                    );
                    properties.insert(
                        "緯度".to_string(),
                        serde_json::Value::String(lat_str.clone()),
                    );
                }

                if let Some(lon_str) = &place.longitude {
                    properties.insert(
                        "longitude".to_string(),
                        serde_json::Value::String(lon_str.clone()),
                    );
                    properties.insert(
                        "經度".to_string(),
                        serde_json::Value::String(lon_str.clone()),
                    );
                }

                if let Some(place_name) = &place.place_name {
                    properties.insert(
                        "place_name".to_string(),
                        serde_json::Value::String(place_name.clone()),
                    );
                    properties.insert(
                        "地點名稱".to_string(),
                        serde_json::Value::String(place_name.clone()),
                    );
                }

                if let Some(rating) = &place.rating {
                    properties.insert(
                        "rating".to_string(),
                        serde_json::Value::String(rating.clone()),
                    );
                    properties.insert(
                        "星級評分".to_string(),
                        serde_json::Value::String(rating.clone()),
                    );
                }

                if let Some(website) = &place.website {
                    properties.insert(
                        "website".to_string(),
                        serde_json::Value::String(website.clone()),
                    );
                    properties.insert(
                        "網站".to_string(),
                        serde_json::Value::String(website.clone()),
                    );
                }

                if let Some(description) = &place.description {
                    properties.insert(
                        "description".to_string(),
                        serde_json::Value::String(description.clone()),
                    );
                    properties.insert(
                        "簡介".to_string(),
                        serde_json::Value::String(description.clone()),
                    );
                }

                if let Some(original_name) = &place.original_name {
                    properties.insert(
                        "original_name".to_string(),
                        serde_json::Value::String(original_name.clone()),
                    );
                    properties.insert(
                        "原文名稱".to_string(),
                        serde_json::Value::String(original_name.clone()),
                    );
                }

                if let Some(english_name) = &place.english_name {
                    properties.insert(
                        "english_name".to_string(),
                        serde_json::Value::String(english_name.clone()),
                    );
                    properties.insert(
                        "英文名稱".to_string(),
                        serde_json::Value::String(english_name.clone()),
                    );
                }

                Some(Feature {
                    bbox: None,
                    geometry: Some(geometry),
                    id: None,
                    properties: Some(properties),
                    foreign_members: None,
                })
            })
            .collect();

        FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        }
    }

    pub fn to_umap_geojson(places: &[GooglePlace]) -> FeatureCollection {
        let features: Vec<Feature> = places
            .iter()
            .filter_map(|place| {
                let lat = place.latitude.as_ref().and_then(|s| s.parse::<f64>().ok());
                let lon = place.longitude.as_ref().and_then(|s| s.parse::<f64>().ok());

                if lat.is_none() && lon.is_none() {
                    return None;
                }

                let geometry = if let (Some(latitude), Some(longitude)) = (lat, lon) {
                    Geometry::new(Value::Point(vec![longitude, latitude]))
                } else {
                    return None;
                };

                let mut properties = Map::new();

                let name = place
                    .title
                    .as_deref()
                    .or(place.place_name.as_deref())
                    .unwrap_or("");
                properties.insert(
                    "name".to_string(),
                    serde_json::Value::String(name.to_string()),
                );

                let mut desc_lines = Vec::new();

                if let Some(t) = &place.title {
                    desc_lines.push(format!("名稱: {}", t));
                }

                if let Some(eng) = &place.english_name
                    && !eng.is_empty()
                {
                    desc_lines.push(format!("English: {}", eng));
                }

                if let Some(url) = &place.url
                    && !url.is_empty()
                {
                    desc_lines.push(format!("Google Maps: {}", url));
                }

                if let Some(rating) = &place.rating
                    && !rating.is_empty()
                {
                    desc_lines.push(format!("Rating: {}", rating));
                }

                if let Some(note) = &place.notes
                    && !note.is_empty()
                {
                    desc_lines.push(format!("Note: {}", note));
                }

                if let Some(tags) = &place.tags
                    && !tags.is_empty()
                {
                    desc_lines.push(format!("Tags: {}", tags));
                }

                if let Some(website) = &place.website
                    && !website.is_empty()
                {
                    desc_lines.push(format!("Website: {}", website));
                }

                if let Some(desc) = &place.description
                    && !desc.is_empty()
                {
                    desc_lines.push(format!("簡介: {}", desc));
                }

                properties.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc_lines.join("\n")),
                );

                Some(Feature {
                    bbox: None,
                    geometry: Some(geometry),
                    id: None,
                    properties: Some(properties),
                    foreign_members: None,
                })
            })
            .collect();

        FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        }
    }
}
