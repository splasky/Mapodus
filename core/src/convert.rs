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
                    .place_name
                    .as_deref()
                    .or(place.title.as_deref())
                    .unwrap_or("");
                properties.insert(
                    "name".to_string(),
                    serde_json::Value::String(name.to_string()),
                );
                insert_string_property(&mut properties, "title", &place.title);
                insert_string_property(&mut properties, "標題", &place.title);
                insert_string_property(&mut properties, "notes", &place.notes);
                insert_string_property(&mut properties, "筆記", &place.notes);
                insert_string_property(&mut properties, "url", &place.url);
                insert_string_property(&mut properties, "網址", &place.url);
                insert_string_property(&mut properties, "tags", &place.tags);
                insert_string_property(&mut properties, "標籤", &place.tags);
                insert_string_property(&mut properties, "comments", &place.comments);
                insert_string_property(&mut properties, "留言", &place.comments);
                insert_string_property(&mut properties, "latitude", &place.latitude);
                insert_string_property(&mut properties, "緯度", &place.latitude);
                insert_string_property(&mut properties, "longitude", &place.longitude);
                insert_string_property(&mut properties, "經度", &place.longitude);
                insert_string_property(&mut properties, "place_name", &place.place_name);
                insert_string_property(&mut properties, "地點名稱", &place.place_name);
                insert_string_property(&mut properties, "rating", &place.rating);
                insert_string_property(&mut properties, "星級評分", &place.rating);
                insert_string_property(&mut properties, "website", &place.website);
                insert_string_property(&mut properties, "網站", &place.website);
                insert_string_property(&mut properties, "poi_description", &place.description);
                insert_string_property(&mut properties, "簡介", &place.description);
                insert_string_property(&mut properties, "english_name", &place.english_name);
                insert_string_property(&mut properties, "英文名稱", &place.english_name);

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

                // Add all individual fields as separate properties for uMap data table
                // Always include every key (empty string when None) so uMap shows all columns
                macro_rules! set_prop {
                    ($en:expr, $val:expr) => {
                        let v = $val.as_deref().unwrap_or("");
                        properties
                            .insert($en.to_string(), serde_json::Value::String(v.to_string()));
                    };
                }
                set_prop!("title", place.title);
                set_prop!("notes", place.notes);
                set_prop!("url", place.url);
                set_prop!("tags", place.tags);
                set_prop!("comments", place.comments);
                set_prop!("latitude", place.latitude);
                set_prop!("longitude", place.longitude);
                set_prop!("place_name", place.place_name);
                set_prop!("rating", place.rating);
                set_prop!("website", place.website);
                set_prop!("original_name", place.original_name);
                set_prop!("english_name", place.english_name);

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

fn insert_string_property(
    properties: &mut Map<String, serde_json::Value>,
    key: &str,
    value: &Option<String>,
) {
    if let Some(value) = value
        && !value.is_empty()
    {
        properties.insert(key.to_string(), serde_json::Value::String(value.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::google::GooglePlace;

    fn make_place(
        title: Option<&str>,
        latitude: Option<&str>,
        longitude: Option<&str>,
    ) -> GooglePlace {
        GooglePlace {
            title: title.map(|s| s.to_string()),
            notes: None,
            url: Some("https://maps.google.com/".to_string()),
            tags: None,
            comments: None,
            latitude: latitude.map(|s| s.to_string()),
            longitude: longitude.map(|s| s.to_string()),
            place_name: None,
            rating: None,
            website: None,
            description: None,
            original_name: None,
            english_name: None,
            place_id: None,
        }
    }

    #[test]
    fn test_to_umap_geojson_all_english_fields_present() {
        let place = GooglePlace {
            title: Some("Test Place".to_string()),
            notes: None,
            url: Some("https://maps.google.com/".to_string()),
            tags: None,
            comments: None,
            latitude: Some("25.033".to_string()),
            longitude: Some("121.565".to_string()),
            place_name: None,
            rating: None,
            website: None,
            description: None,
            original_name: None,
            english_name: None,
            place_id: Some("ChIJabc".to_string()),
        };
        let fc = Converter::to_umap_geojson(&[place]);
        assert_eq!(fc.features.len(), 1);
        let props = fc.features[0].properties.as_ref().unwrap();

        let english_keys = [
            "title",
            "notes",
            "url",
            "tags",
            "comments",
            "latitude",
            "longitude",
            "place_name",
            "rating",
            "website",
            "original_name",
            "english_name",
        ];
        for key in &english_keys {
            assert!(props.contains_key(*key), "Missing key: {}", key);
            assert!(
                props.get(*key).and_then(|v| v.as_str()).is_some(),
                "Key {} is not a string",
                key
            );
        }

        // Fields that were Some
        assert_eq!(
            props.get("title").and_then(|v| v.as_str()),
            Some("Test Place")
        );
        assert_eq!(
            props.get("url").and_then(|v| v.as_str()),
            Some("https://maps.google.com/")
        );
        assert_eq!(
            props.get("latitude").and_then(|v| v.as_str()),
            Some("25.033")
        );
        assert_eq!(
            props.get("longitude").and_then(|v| v.as_str()),
            Some("121.565")
        );

        // Fields that were None → empty strings
        assert_eq!(props.get("notes").and_then(|v| v.as_str()), Some(""));
        assert_eq!(props.get("tags").and_then(|v| v.as_str()), Some(""));
        assert_eq!(props.get("comments").and_then(|v| v.as_str()), Some(""));
        assert_eq!(props.get("place_name").and_then(|v| v.as_str()), Some(""));
        assert_eq!(props.get("rating").and_then(|v| v.as_str()), Some(""));
        assert_eq!(props.get("website").and_then(|v| v.as_str()), Some(""));
        assert_eq!(
            props.get("original_name").and_then(|v| v.as_str()),
            Some("")
        );
        assert_eq!(props.get("english_name").and_then(|v| v.as_str()), Some(""));
    }

    #[test]
    fn test_to_umap_geojson_name_falls_back_to_place_name() {
        let place = make_place(None, Some("25.0"), Some("121.0"));
        let fc = Converter::to_umap_geojson(&[place]);
        let props = fc.features[0].properties.as_ref().unwrap();
        // name should be empty since both title and place_name are None
        assert_eq!(props.get("name").and_then(|v| v.as_str()), Some(""));
    }

    #[test]
    fn test_to_geojson_skips_fields_when_none() {
        let place = make_place(Some("Name"), Some("25.0"), Some("121.0"));
        let fc = Converter::to_geojson(&[place]);
        assert_eq!(fc.features.len(), 1);
        let props = fc.features[0].properties.as_ref().unwrap();
        // to_geojson omits None fields entirely
        assert!(props.contains_key("title"));
        assert!(!props.contains_key("notes"));
    }

    #[test]
    fn test_to_geojson_skips_feature_without_coords() {
        let place = make_place(Some("No coords"), None, None);
        let fc = Converter::to_geojson(&[place]);
        assert_eq!(fc.features.len(), 0);
    }

    #[test]
    fn test_to_umap_geojson_skips_feature_without_coords() {
        let place = make_place(Some("No coords"), None, None);
        let fc = Converter::to_umap_geojson(&[place]);
        assert_eq!(fc.features.len(), 0);
    }
}
