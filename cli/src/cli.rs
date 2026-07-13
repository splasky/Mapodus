// Copyright 2026 HYChang
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

use clap::Parser;
use std::path::Path;

#[derive(Parser)]
#[command(name = "mapodus", about = "Convert Google Maps saved places to uMap")]
pub struct CliArgs {
    #[arg(short = 't', long, help = "Path to Google Takeout CSV or JSON file")]
    pub takeout: Option<String>,

    #[arg(
        short = 'g',
        long,
        help = "Path to existing GeoJSON file (alternative to --takeout)"
    )]
    pub geojson: Option<String>,

    #[arg(
        short = 'o',
        long,
        help = "Output GeoJSON file path (skip uMap upload)"
    )]
    pub output: Option<String>,

    #[arg(
        long,
        default_value = "https://umap.openstreetmap.fr/en/",
        help = "uMap instance URL"
    )]
    pub umap_url: String,

    #[arg(long, help = "uMap map ID to upload to")]
    pub umap_map_id: Option<String>,

    #[arg(long, help = "Create a new map with this name before uploading")]
    pub create_map: Option<String>,

    #[arg(
        long,
        help = "uMap session cookie (format: sessionid=xxx; csrftoken=xxx)"
    )]
    pub umap_cookie: Option<String>,

    #[arg(long, default_value = "Google Maps Saved", help = "Target layer name")]
    pub layer_name: String,
}

impl CliArgs {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        let has_takeout = self.takeout.is_some();
        let has_geojson = self.geojson.is_some();

        if !has_takeout && !has_geojson {
            return Err(anyhow::anyhow!(
                "Either --takeout or --geojson must be provided"
            ));
        }

        if has_takeout && has_geojson {
            return Err(anyhow::anyhow!(
                "Only one of --takeout or --geojson can be provided"
            ));
        }

        if self.create_map.is_some() && self.umap_cookie.is_none() {
            return Err(anyhow::anyhow!(
                "--umap-cookie must be provided when --create-map is specified"
            ));
        }

        if self.create_map.is_some() && self.umap_map_id.is_some() {
            return Err(anyhow::anyhow!(
                "--create-map and --umap-map-id are mutually exclusive"
            ));
        }

        if self.umap_map_id.is_some() && self.umap_cookie.is_none() {
            return Err(anyhow::anyhow!(
                "--umap-cookie must be provided when --umap-map-id is specified"
            ));
        }

        if let Some(ref path) = self.takeout
            && !Path::new(path).exists()
        {
            return Err(anyhow::anyhow!("Takeout file does not exist: {}", path));
        }

        if let Some(ref path) = self.geojson
            && !Path::new(path).exists()
        {
            return Err(anyhow::anyhow!("GeoJSON file does not exist: {}", path));
        }

        if let Some(ref path) = self.output {
            let parent = Path::new(path).parent().unwrap_or(Path::new("."));
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Output directory does not exist: {:?}",
                    parent
                ));
            }
        }

        Ok(())
    }
}
