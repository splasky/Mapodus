use anyhow::Result;
use clap::Parser;

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::CliArgs::parse();
    args.validate()?;

    let feature_collection = if let Some(takeout_path) = &args.takeout {
        let places = umap_core::google::parse_takeout(takeout_path)?;

        if args.umap_map_id.is_some() || args.create_map.is_some() {
            umap_core::convert::Converter::to_umap_geojson(&places)
        } else {
            umap_core::convert::Converter::to_geojson(&places)
        }
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

    let cookie_str = args.umap_cookie.as_ref();
    let auth = cookie_str.map(|s| umap_core::umap::CookieAuth::from_cookie_str(s).unwrap());

    let map_id: String;

    if let Some(new_map_name) = &args.create_map {
        let auth = auth
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("umap-cookie required for --create-map"))?;
        let client = umap_core::umap::UmapClient::new(&args.umap_url);
        map_id = client
            .create_map(new_map_name, &feature_collection, auth)
            .await?
            .id;
    } else if let Some(existing_id) = &args.umap_map_id {
        map_id = existing_id.clone();
    } else {
        return Ok(());
    }

    if let Some(auth) = &auth {
        let client = umap_core::umap::UmapClient::new(&args.umap_url);

        let layer_id = match client
            .find_or_create_layer(&map_id, &args.layer_name, auth)
            .await
        {
            Ok(id) => {
                client
                    .upload_geojson(&map_id, &id, &args.layer_name, &feature_collection, auth)
                    .await?;
                id
            }
            Err(_) => {
                let layer_id = client
                    .create_and_upload_layer(&map_id, &args.layer_name, &feature_collection, auth)
                    .await?;
                layer_id
            }
        };

        println!(
            "Successfully uploaded to uMap map {} layer {}",
            map_id, layer_id
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use umap_core::convert::Converter;
    use umap_core::google;

    fn test_data_path(filename: &str) -> String {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("examples");
        base.join(filename).to_string_lossy().to_string()
    }

    #[test]
    fn test_parse_original_csv() {
        let path = test_data_path("2026北海道.csv");
        let places = google::parse_takeout(&path).unwrap();
        assert_eq!(places.len(), 49);
        let first = &places[0];
        assert_eq!(first.title.as_deref(), Some("MYSTAYS 札幌公園精品酒店"));
        assert!(
            first
                .url
                .as_deref()
                .unwrap_or("")
                .starts_with("https://www.google.com/maps/")
        );
    }

    #[test]
    fn test_parse_updated_csv() {
        let path = test_data_path("2026北海道_updated.csv");
        let places = google::parse_takeout(&path).unwrap();
        assert_eq!(places.len(), 49);
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
        assert_eq!(fc.features.len(), 49);

        let first_feature = &fc.features[0];
        let props = first_feature.properties.as_ref().unwrap();

        assert_eq!(
            props.get("name").and_then(|v| v.as_str()),
            Some("MYSTAYS 札幌公園精品酒店")
        );

        let desc = props
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(desc.contains("名稱: MYSTAYS 札幌公園精品酒店"));
        assert!(desc.contains("English: HOTEL MYSTAYS PREMIER Sapporo Park"));

        let geometry = first_feature.geometry.as_ref().unwrap();
        match &geometry.value {
            geojson::Value::Point(coords) => {
                assert!((coords[0] - 141.3569474).abs() < 1e-7);
                assert!((coords[1] - 43.0495311).abs() < 1e-7);
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

        assert_eq!(fc.features.len(), expected.features.len());

        if let (Some(first_fc), Some(first_exp)) = (fc.features.first(), expected.features.first())
        {
            let fc_coords = first_fc.geometry.as_ref().and_then(|g| match &g.value {
                geojson::Value::Point(c) => Some(c),
                _ => None,
            });
            let exp_coords = first_exp.geometry.as_ref().and_then(|g| match &g.value {
                geojson::Value::Point(c) => Some(c),
                _ => None,
            });
            assert_eq!(fc_coords, exp_coords);
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
            assert_eq!(fc_coords, exp_coords);
        }
    }
}
