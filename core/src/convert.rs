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

                // GeoJSON point geometry needs both numbers. Drop incomplete
                // rows here so upload code only sees valid map features.
                if lat.is_none() && lon.is_none() {
                    return None;
                }

                let geometry = if let (Some(latitude), Some(longitude)) = (lat, lon) {
                    Geometry::new(Value::Point(vec![longitude, latitude]))
                } else {
                    return None;
                };

                let mut properties = Map::new();

                // Preserve both normalized English keys and localized Takeout
                // labels so downstream tools can use either convention.
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

                insert_google_place_properties(&mut properties, &place.google_place_details);

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

                // uMap displays `name` as the marker label. Prefer the user's
                // saved-list title, then fall back to the Google place name.
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
                insert_string_property(&mut properties, "original_name", &place.original_name);
                insert_string_property(&mut properties, "原文名稱", &place.original_name);
                insert_google_place_properties(&mut properties, &place.google_place_details);

                // uMap renders `description` in the marker popup, so collect
                // the user-facing details there while still keeping individual
                // fields below for table/filter use.
                let mut desc_lines = Vec::new();

                if let Some(name) = place.place_name.as_ref().or(place.title.as_ref())
                    && !name.is_empty()
                {
                    desc_lines.push(format!("名稱: {}", name));
                }

                if let Some(original_name) = &place.original_name
                    && !original_name.is_empty()
                {
                    desc_lines.push(format!("原始名稱: {}", original_name));
                }

                if let Some(address) = google_place_string(
                    &place.google_place_details,
                    &["formattedAddress"],
                )
                .or_else(|| {
                    google_place_string(&place.google_place_details, &["shortFormattedAddress"])
                }) {
                    desc_lines.push(format!("地址: {}", address));
                }

                if let Some(primary_type) = google_place_string(
                    &place.google_place_details,
                    &["primaryTypeDisplayName", "text"],
                )
                .or_else(|| google_place_string(&place.google_place_details, &["primaryType"]))
                {
                    desc_lines.push(format!("類型: {}", primary_type));
                }

                if let Some(status) =
                    google_place_string(&place.google_place_details, &["businessStatus"])
                {
                    desc_lines.push(format!("狀態: {}", status));
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

fn insert_google_place_properties(
    properties: &mut Map<String, serde_json::Value>,
    details: &Option<serde_json::Value>,
) {
    let Some(details) = details else {
        return;
    };

    properties.insert("google_place_details".to_string(), details.clone());

    insert_google_value_property(properties, "google_place_id", details, &["id"]);
    insert_google_value_property(properties, "google_place_resource_name", details, &["name"]);
    insert_google_value_property(
        properties,
        "google_place_display_name",
        details,
        &["displayName", "text"],
    );
    insert_google_value_property(
        properties,
        "google_place_display_language",
        details,
        &["displayName", "languageCode"],
    );
    insert_google_value_property(
        properties,
        "google_formatted_address",
        details,
        &["formattedAddress"],
    );
    insert_google_value_property(
        properties,
        "google_short_formatted_address",
        details,
        &["shortFormattedAddress"],
    );
    insert_google_value_property(properties, "google_primary_type", details, &["primaryType"]);
    insert_google_value_property(
        properties,
        "google_primary_type_display_name",
        details,
        &["primaryTypeDisplayName", "text"],
    );
    insert_google_value_property(
        properties,
        "google_primary_type_display_language",
        details,
        &["primaryTypeDisplayName", "languageCode"],
    );
    insert_google_value_property(
        properties,
        "google_business_status",
        details,
        &["businessStatus"],
    );
    insert_google_value_property(properties, "google_maps_uri", details, &["googleMapsUri"]);
    insert_google_value_property(
        properties,
        "google_maps_links",
        details,
        &["googleMapsLinks"],
    );
    insert_google_value_property(properties, "google_types", details, &["types"]);
    insert_google_value_property(properties, "google_plus_code", details, &["plusCode"]);
    insert_google_value_property(properties, "google_viewport", details, &["viewport"]);
    insert_google_value_property(
        properties,
        "google_address_components",
        details,
        &["addressComponents"],
    );
    insert_google_value_property(
        properties,
        "google_postal_address",
        details,
        &["postalAddress"],
    );
    insert_google_value_property(
        properties,
        "google_accessibility_options",
        details,
        &["accessibilityOptions"],
    );
    insert_google_value_property(
        properties,
        "google_containing_places",
        details,
        &["containingPlaces"],
    );
    insert_google_value_property(
        properties,
        "google_icon_background_color",
        details,
        &["iconBackgroundColor"],
    );
    insert_google_value_property(
        properties,
        "google_icon_mask_base_uri",
        details,
        &["iconMaskBaseUri"],
    );
    insert_google_value_property(properties, "google_opening_date", details, &["openingDate"]);
    insert_google_value_property(
        properties,
        "google_pure_service_area_business",
        details,
        &["pureServiceAreaBusiness"],
    );
    insert_google_value_property(
        properties,
        "google_sub_destinations",
        details,
        &["subDestinations"],
    );
    insert_google_value_property(properties, "google_time_zone", details, &["timeZone"]);
    insert_google_value_property(
        properties,
        "google_utc_offset_minutes",
        details,
        &["utcOffsetMinutes"],
    );
    insert_google_value_property(properties, "google_photos", details, &["photos"]);
    insert_google_value_property(
        properties,
        "google_attributions",
        details,
        &["attributions"],
    );
    insert_google_value_property(properties, "google_moved_place", details, &["movedPlace"]);
    insert_google_value_property(
        properties,
        "google_moved_place_id",
        details,
        &["movedPlaceId"],
    );
}

fn insert_google_value_property(
    properties: &mut Map<String, serde_json::Value>,
    key: &str,
    details: &serde_json::Value,
    path: &[&str],
) {
    if let Some(value) = google_place_value(details, path)
        && !value.is_null()
    {
        properties.insert(key.to_string(), value.clone());
    }
}

fn google_place_string(details: &Option<serde_json::Value>, path: &[&str]) -> Option<String> {
    google_place_value(details.as_ref()?, path)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn google_place_value<'a>(
    details: &'a serde_json::Value,
    path: &[&str],
) -> Option<&'a serde_json::Value> {
    path.iter()
        .try_fold(details, |current, key| current.get(*key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place_with_all_detail_fields() -> GooglePlace {
        GooglePlace {
            title: Some("Saved title".to_string()),
            notes: Some("Note".to_string()),
            url: Some("https://www.google.com/maps/place/?q=place_id:ChIJSANITIZED".to_string()),
            tags: Some("tag".to_string()),
            comments: Some("comment".to_string()),
            latitude: Some("24.1".to_string()),
            longitude: Some("120.2".to_string()),
            place_name: Some("API display name".to_string()),
            rating: Some("4.7".to_string()),
            website: Some("https://example.com".to_string()),
            description: Some("API description".to_string()),
            original_name: Some("Saved title".to_string()),
            place_id: Some("ChIJSANITIZED".to_string()),
            google_place_details: Some(serde_json::json!({
                "id": "places/ChIJSANITIZED",
                "name": "places/ChIJSANITIZED",
                "displayName": {"text": "API display name", "languageCode": "zh-TW"},
                "formattedAddress": "測試地址",
                "shortFormattedAddress": "短地址",
                "location": {"latitude": 24.1, "longitude": 120.2},
                "businessStatus": "OPERATIONAL",
                "primaryType": "restaurant",
                "primaryTypeDisplayName": {"text": "餐廳", "languageCode": "zh-TW"},
                "googleMapsUri": "https://maps.google.com/?cid=123",
                "types": ["restaurant", "food"],
                "plusCode": {"globalCode": "TEST+CODE"},
                "utcOffsetMinutes": 480
            })),
        }
    }

    #[test]
    fn geojson_export_omits_english_name_columns() {
        let fc = Converter::to_geojson(&[place_with_all_detail_fields()]);
        let properties = fc.features[0].properties.as_ref().unwrap();

        assert!(properties.contains_key("original_name"));
        assert!(properties.contains_key("原文名稱"));
        assert!(properties.contains_key("google_place_details"));
        assert_eq!(
            properties
                .get("google_place_display_name")
                .and_then(serde_json::Value::as_str),
            Some("API display name")
        );
        assert_eq!(
            properties
                .get("google_primary_type_display_name")
                .and_then(serde_json::Value::as_str),
            Some("餐廳")
        );
        assert!(!properties.contains_key("english_name"));
        assert!(!properties.contains_key("英文名稱"));
    }

    #[test]
    fn umap_export_matches_csv_detail_columns_without_english_name() {
        let fc = Converter::to_umap_geojson(&[place_with_all_detail_fields()]);
        let properties = fc.features[0].properties.as_ref().unwrap();

        assert!(properties.contains_key("original_name"));
        assert!(properties.contains_key("原文名稱"));
        assert_eq!(
            properties
                .get("google_formatted_address")
                .and_then(serde_json::Value::as_str),
            Some("測試地址")
        );
        assert_eq!(
            properties
                .get("google_business_status")
                .and_then(serde_json::Value::as_str),
            Some("OPERATIONAL")
        );
        assert!(!properties.contains_key("english_name"));
        assert!(!properties.contains_key("英文名稱"));
        let description = properties["description"].as_str().unwrap();
        assert!(description.contains("名稱: API display name"));
        assert!(description.contains("地址: 測試地址"));
        assert!(description.contains("類型: 餐廳"));
        assert!(description.contains("狀態: OPERATIONAL"));
    }
}
