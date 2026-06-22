// Copyright 2025 google-maps-to-umap Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use csv;
use serde_json;
use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
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
}

impl GooglePlace {
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
        };

        let header_map = headers
            .iter()
            .enumerate()
            .map(|(i, header)| (header.to_lowercase(), i))
            .collect::<std::collections::HashMap<String, usize>>();

        macro_rules! set_field {
            ($field:ident, $header:expr) => {
                if let Some(idx) = header_map.get(&$header.to_lowercase()) {
                    place.$field = record.get(*idx).map(|s| s.to_string());
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

        place
    }

    pub fn from_geojson_feature(feature: &serde_json::Value) -> Self {
        let empty_map = serde_json::Map::new();
        let properties = feature.get("properties").and_then(|v| v.as_object()).unwrap_or(&empty_map);

        GooglePlace {
            title: properties.get("Title").and_then(|v| v.as_str()).map(|s| s.to_string()),
            notes: properties.get("Notes").and_then(|v| v.as_str()).map(|s| s.to_string()),
            url: properties.get("URL").and_then(|v| v.as_str()).map(|s| s.to_string()),
            tags: properties.get("Tags").and_then(|v| v.as_str()).map(|s| s.to_string()),
            comments: properties.get("Comments").and_then(|v| v.as_str()).map(|s| s.to_string()),
            latitude: properties.get("Latitude").and_then(|v| v.as_str()).map(|s| s.to_string()),
            longitude: properties.get("Longitude").and_then(|v| v.as_str()).map(|s| s.to_string()),
            place_name: properties.get("Place Name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            rating: properties.get("Rating").and_then(|v| v.as_str()).map(|s| s.to_string()),
            website: properties.get("Website").and_then(|v| v.as_str()).map(|s| s.to_string()),
            description: properties.get("Description").and_then(|v| v.as_str()).map(|s| s.to_string()),
            original_name: properties.get("Original Name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            english_name: properties.get("English Name").and_then(|v| v.as_str()).map(|s| s.to_string()),
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
        _ => Err(anyhow!("Unsupported file format. Expected .csv, .json, or .geojson")),
    }
}
