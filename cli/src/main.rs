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
                client
                    .create_and_upload_layer(&map_id, &args.layer_name, &feature_collection, auth)
                    .await?
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
        let path = test_data_path("test.csv");
        let places = google::parse_takeout(&path).unwrap();
        assert_eq!(places.len(), 4);
        let first = &places[0];
        assert_eq!(
            first.title.as_deref(),
            Some("6owl door Hsinchu Dongnan Branch")
        );
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
        let path = test_data_path("test_updated.csv");
        let places = google::parse_takeout(&path).unwrap();
        assert_eq!(places.len(), 4);
        let first = &places[0];
        assert_eq!(first.latitude.as_deref(), Some("24.7994433"));
        assert_eq!(first.longitude.as_deref(), Some("120.9730098"));
        assert_eq!(first.rating.as_deref(), Some("4"));
        assert_eq!(
            first.english_name.as_deref(),
            Some("6owl door Hsinchu Dongnan Branch")
        );
    }

    #[test]
    fn test_convert_to_umap_geojson() {
        let path = test_data_path("test_updated.csv");
        let places = google::parse_takeout(&path).unwrap();
        let fc = Converter::to_umap_geojson(&places);
        assert_eq!(fc.features.len(), 4);

        let first_feature = &fc.features[0];
        let props = first_feature.properties.as_ref().unwrap();

        assert_eq!(
            props.get("name").and_then(|v| v.as_str()),
            Some("六扇門時尚湯鍋 新竹東南店")
        );
        assert_eq!(
            props.get("title").and_then(|v| v.as_str()),
            Some("6owl door Hsinchu Dongnan Branch")
        );
        assert_eq!(
            props.get("place_name").and_then(|v| v.as_str()),
            Some("六扇門時尚湯鍋 新竹東南店")
        );
        assert_eq!(props.get("rating").and_then(|v| v.as_str()), Some("4"));
        assert_eq!(
            props.get("english_name").and_then(|v| v.as_str()),
            Some("6owl door Hsinchu Dongnan Branch")
        );
        assert!(props.get("website").and_then(|v| v.as_str()).is_some());

        let desc = props
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(desc.contains("名稱: 6owl door Hsinchu Dongnan Branch"));
        assert!(desc.contains("English: 6owl door Hsinchu Dongnan Branch"));

        let geometry = first_feature.geometry.as_ref().unwrap();
        match &geometry.value {
            geojson::Value::Point(coords) => {
                assert!((coords[0] - 120.9730098).abs() < 1e-7);
                assert!((coords[1] - 24.7994433).abs() < 1e-7);
            }
            _ => panic!("expected Point geometry"),
        }
    }

    #[test]
    fn test_convert_matches_expected_geojson() {
        let path = test_data_path("test_updated.csv");
        let places = google::parse_takeout(&path).unwrap();
        let fc = Converter::to_umap_geojson(&places);

        let expected_path = test_data_path("test.geojson");
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

    // ── Takeout-specific enrichment tests ──

    #[test]
    fn test_parse_takeout_csv_place_id_extracted() {
        let path = test_data_path("test.csv");
        let places = google::parse_takeout(&path).unwrap();

        // Rows 1-3 (0-indexed from parsed) have place URLs with !1s protobuf format
        // Row 0: 6owl door → place_id = 0x346835e9aa147b0b:0x8e09cb932ab96f34
        let p0 = &places[0];
        assert_eq!(
            p0.place_id.as_deref(),
            Some("0x346835e9aa147b0b:0x8e09cb932ab96f34")
        );

        // Row 1: 新豐車站
        let p1 = &places[1];
        assert_eq!(
            p1.place_id.as_deref(),
            Some("0x346831505ce20595:0x1d13efe96c43c466")
        );

        // Row 2: 明新科技大學
        let p2 = &places[2];
        assert_eq!(
            p2.place_id.as_deref(),
            Some("0x34683154faa8283b:0x92cb1c5564a574ef")
        );

        // Row 3: /search/lat,lng URL → no place_id
        let p3 = &places[3];
        assert_eq!(p3.place_id, None);
    }

    #[test]
    fn test_parse_takeout_csv_coords_extracted_from_search_url() {
        let path = test_data_path("test.csv");
        let places = google::parse_takeout(&path).unwrap();

        // First 3 rows have !1s URLs without @lat,lng → coords should be None
        for i in 0..3 {
            let p = &places[i];
            assert!(
                p.latitude.is_none(),
                "Row {} should not have latitude (no @ in URL), got {:?}",
                i,
                p.latitude
            );
            assert!(
                p.longitude.is_none(),
                "Row {} should not have longitude (no @ in URL), got {:?}",
                i,
                p.longitude
            );
        }

        // Row 3 has /search/lat,lng → coords should be extracted
        let p3 = &places[3];
        assert_eq!(p3.latitude.as_deref(), Some("24.8583332"));
        assert_eq!(p3.longitude.as_deref(), Some("120.9927297"));
    }

    #[test]
    fn test_parse_takeout_csv_enrichment_fields_stay_none() {
        let path = test_data_path("test.csv");
        let places = google::parse_takeout(&path).unwrap();

        // test.csv only has: 標題, 筆記, 網址, 標籤, 留言
        // All enrichment fields should be None
        for (i, p) in places.iter().enumerate() {
            assert!(
                p.rating.is_none(),
                "Row {} should have no rating (not in CSV)",
                i
            );
            assert!(
                p.website.is_none(),
                "Row {} should have no website (not in CSV)",
                i
            );
            assert!(
                p.place_name.is_none(),
                "Row {} should have no place_name (not in CSV)",
                i
            );
            assert!(
                p.description.is_none(),
                "Row {} should have no description (not in CSV)",
                i
            );
            assert!(
                p.original_name.is_none(),
                "Row {} should have no original_name (not in CSV)",
                i
            );
            assert!(
                p.english_name.is_none(),
                "Row {} should have no english_name (not in CSV)",
                i
            );
        }
    }

    #[test]
    fn test_parse_takeout_csv_empty_rows_skipped() {
        let path = test_data_path("test.csv");
        let places = google::parse_takeout(&path).unwrap();
        // test.csv has 4 data rows (the blank row after headers is skipped)
        assert_eq!(places.len(), 4);
    }
}
