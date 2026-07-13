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

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{WebviewUrl, WebviewWindowBuilder};

const APP_CONFIG_DIR: &str = "mapodus";
const CONFIG_FILE: &str = "config.toml";
const DEFAULT_UMAP_URL: &str = "https://umap.openstreetmap.fr/en/";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DesktopConfig {
    umap_default_url: String,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            umap_default_url: DEFAULT_UMAP_URL.to_string(),
        }
    }
}

fn main() {
    dotenvy::dotenv().ok();
    // SAFETY: This runs during process startup before the embedded backend is
    // started, so no other application threads read environment variables yet.
    unsafe {
        std::env::set_var("GMAP_TO_UMAP_DESKTOP", "1");
    }
    if let Err(error) = load_desktop_config() {
        eprintln!("Failed to load desktop config: {}", error);
    }

    tauri::Builder::default()
        .setup(|app| {
            let (addr, _backend) = tauri::async_runtime::block_on(web::serve_on_available_port())?;
            let url = url::Url::parse(&format!("http://{addr}/"))?;

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("Mapodus")
                .inner_size(1200.0, 820.0)
                .min_inner_size(900.0, 640.0)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

fn load_desktop_config() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = desktop_config_path()?;
    let config = read_or_create_config(&config_path)?;

    if std::env::var_os("UMAP_DEFAULT_URL").is_none() {
        // SAFETY: This runs during process startup before the embedded backend is
        // started, so no other application threads read environment variables yet.
        unsafe {
            std::env::set_var("UMAP_DEFAULT_URL", config.umap_default_url);
        }
    }

    Ok(())
}

fn desktop_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let base = dirs::config_dir().ok_or("failed to resolve OS config directory")?;
    Ok(base.join(APP_CONFIG_DIR).join(CONFIG_FILE))
}

fn read_or_create_config(path: &PathBuf) -> Result<DesktopConfig, Box<dyn std::error::Error>> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let default_config = DesktopConfig::default();
        fs::write(path, toml::to_string_pretty(&default_config)?)?;
        return Ok(default_config);
    }

    let text = fs::read_to_string(path)?;
    let config: DesktopConfig = toml::from_str(&text)?;
    Ok(config)
}
