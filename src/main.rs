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

use anyhow::Result;
use clap::Parser;

mod cli;
mod convert;
mod error;
mod google;
mod umap;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::CliArgs::parse();
    args.validate()?;

    let feature_collection = if let Some(takeout_path) = &args.takeout {
        let places = google::parse_takeout(takeout_path)?;
        convert::Converter::to_geojson(&places)
    } else if let Some(geojson_path) = &args.geojson {
        let content = std::fs::read_to_string(geojson_path)?;
        serde_json::from_str(&content)?
    } else {
        unreachable!()
    };

    if let Some(output_path) = &args.output {
        let json = serde_json::to_string_pretty(&feature_collection)?;
        std::fs::write(output_path, json)?;
        println!("Saved GeoJSON to {}", output_path);
    }

    if let Some(map_id) = &args.umap_map_id {
        let cookie_str = args.umap_cookie.as_ref().expect("umap-cookie required for upload");
        let auth = umap::CookieAuth::from_cookie_str(cookie_str)?;
        let client = umap::UmapClient::new(&args.umap_url);

        println!("Finding or creating layer '{}' on map {}", args.layer_name, map_id);
        let layer_id = client
            .find_or_create_layer(map_id, &args.layer_name, &auth)
            .await?;
        println!("Layer ID: {}", layer_id);

        println!("Uploading GeoJSON...");
        client
            .upload_geojson(map_id, &layer_id, &args.layer_name, &feature_collection, &auth)
            .await?;
        println!("Successfully uploaded to uMap map {} layer {}", map_id, layer_id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::convert::Converter;
    use crate::google;

    fn test_data_path(filename: &str) -> String {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
        base.join(filename).to_string_lossy().to_string()
    }

    #[test]
    fn test_parse_original_csv() {
        let path = test_data_path("2026北海道.csv");
        let places = google::parse_takeout(&path).unwrap();
        assert_eq!(places.len(), 49, "expected 49 data rows");
        let first = &places[0];
        assert_eq!(
            first.title.as_deref(),
            Some("MYSTAYS 札幌公園精品酒店")
        );
        assert!(
            first.url.as_deref().unwrap_or("").starts_with("https://www.google.com/maps/"),
            "URL should start with expected prefix"
        );
    }

    #[test]
    fn test_parse_updated_csv() {
        let path = test_data_path("2026北海道_updated.csv");
        let places = google::parse_takeout(&path).unwrap();
        assert_eq!(places.len(), 49, "expected 49 data rows");
        let first = &places[0];
        assert_eq!(first.latitude.as_deref(), Some("43.0495311"));
        assert_eq!(first.longitude.as_deref(), Some("141.3569474"));
        assert_eq!(first.rating.as_deref(), Some("4"));
        assert_eq!(
            first.english_name.as_deref(),
            Some("HOTEL MYSTAYS PREMIER Sapporo Park")
        );
    }

    #[test]
    fn test_convert_to_umap_geojson() {
        let path = test_data_path("2026北海道_updated.csv");
        let places = google::parse_takeout(&path).unwrap();
        let fc = Converter::to_umap_geojson(&places);
        assert_eq!(fc.features.len(), 49, "expected 49 features");

        let first_feature = &fc.features[0];
        let props = first_feature.properties.as_ref().unwrap();

        assert_eq!(
            props.get("name").and_then(|v| v.as_str()),
            Some("MYSTAYS 札幌公園精品酒店")
        );

        let desc = props.get("description").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            desc.contains("名稱: MYSTAYS 札幌公園精品酒店"),
            "description should contain 名稱"
        );
        assert!(
            desc.contains("English: HOTEL MYSTAYS PREMIER Sapporo Park"),
            "description should contain English name"
        );

        let geometry = first_feature.geometry.as_ref().unwrap();
        match &geometry.value {
            geojson::Value::Point(coords) => {
                assert!((coords[0] - 141.3569474).abs() < 1e-7, "longitude mismatch");
                assert!((coords[1] - 43.0495311).abs() < 1e-7, "latitude mismatch");
            }
            _ => panic!("expected Point geometry"),
        }
    }

    #[test]
    fn test_convert_matches_expected_geojson() {
        let path = test_data_path("2026北海道_updated.csv");
        let places = google::parse_takeout(&path).unwrap();
        let fc = Converter::to_umap_geojson(&places);

        let expected_path = test_data_path("2026北海道_umap.geojson");
        let content = std::fs::read_to_string(&expected_path).unwrap();
        let expected: geojson::FeatureCollection = serde_json::from_str(&content).unwrap();

        assert_eq!(
            fc.features.len(),
            expected.features.len(),
            "feature count mismatch"
        );

        if let (Some(first_fc), Some(first_exp)) = (fc.features.first(), expected.features.first()) {
            let fc_coords = first_fc.geometry.as_ref().and_then(|g| match &g.value {
                geojson::Value::Point(c) => Some(c),
                _ => None,
            });
            let exp_coords = first_exp.geometry.as_ref().and_then(|g| match &g.value {
                geojson::Value::Point(c) => Some(c),
                _ => None,
            });
            assert_eq!(fc_coords, exp_coords, "first feature coords mismatch");
        }

        if let (Some(last_fc), Some(last_exp)) = (fc.features.last(), expected.features.last()) {
            let fc_coords = last_fc.geometry.as_ref().and_then(|g| match &g.value {
                geojson::Value::Point(c) => Some(c),
                _ => None,
            });
            let exp_coords = last_exp.geometry.as_ref().and_then(|g| match &g.value {
                geojson::Value::Point(c) => Some(c),
                _ => None,
            });
            assert_eq!(fc_coords, exp_coords, "last feature coords mismatch");
        }
    }
}
