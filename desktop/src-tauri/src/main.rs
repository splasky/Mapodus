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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_ID: AtomicU32 = AtomicU32::new(0);

    fn test_config_path() -> (std::path::PathBuf, std::path::PathBuf) {
        let id = TEST_ID.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!("gmap-test-{}-{}", std::process::id(), id));
        (tmp.clone(), tmp.join("config.toml"))
    }

    #[test]
    fn desktop_config_default_url() {
        let config = DesktopConfig::default();
        assert!(!config.umap_default_url.is_empty());
        assert!(config.umap_default_url.starts_with("http"));
    }

    #[test]
    fn desktop_config_path_ends_with_expected() {
        let path = desktop_config_path().unwrap();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("gmap-to-umap"));
        assert!(path_str.contains("config.toml"));
    }

    #[test]
    fn read_or_create_config_creates_default_when_missing() {
        let (tmp, config_path) = test_config_path();

        let config = read_or_create_config(&config_path).unwrap();
        assert!(!config.umap_default_url.is_empty());
        assert!(config.umap_default_url.starts_with("http"));
        assert!(config_path.exists());

        let _ = std::fs::remove_file(&config_path);
        let _ = std::fs::remove_dir(&tmp);
    }

    #[test]
    fn read_or_create_config_reads_existing() {
        let (tmp, config_path) = test_config_path();
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            &config_path,
            r#"umap_default_url = "http://localhost:8000/en/""#,
        )
        .unwrap();

        let config = read_or_create_config(&config_path).unwrap();
        assert_eq!(config.umap_default_url, "http://localhost:8000/en/");

        let _ = std::fs::remove_file(&config_path);
        let _ = std::fs::remove_dir(&tmp);
    }
}
