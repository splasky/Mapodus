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
