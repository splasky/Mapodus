use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const APP_CONFIG_DIR: &str = "mapodus";
const CONFIG_FILE: &str = "config.toml";
const DEFAULT_UMAP_URL: &str = "https://umap.openstreetmap.fr/en/";
const DEFAULT_LOCALE: &str = "en";
const KEYCHAIN_SERVICE: &str = "com.splasky.mapodus";
const KEYCHAIN_UMAP_PASSWORD: &str = "umap_password";
const KEYCHAIN_GOOGLE_MAPS_API_KEY: &str = "google_maps_api_key";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub umap_default_url: String,
    pub umap_account: Option<String>,
    pub locale: String,
    pub dev_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            umap_default_url: DEFAULT_UMAP_URL.to_string(),
            umap_account: None,
            locale: DEFAULT_LOCALE.to_string(),
            dev_mode: false,
        }
    }
}

pub fn is_desktop_mode() -> bool {
    std::env::var("GMAP_TO_UMAP_DESKTOP").as_deref() == Ok("1")
}

pub fn load_settings() -> AppSettings {
    read_settings().unwrap_or_else(|error| {
        eprintln!("Failed to read settings: {error}");
        AppSettings::default()
    })
}

pub fn save_settings(settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, toml::to_string_pretty(settings)?)?;
    Ok(())
}

pub fn default_umap_url() -> String {
    std::env::var("UMAP_DEFAULT_URL")
        .or_else(|_| std::env::var("UMAP_URL"))
        .ok()
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| load_settings().umap_default_url)
}

pub fn google_maps_api_key_from_keychain() -> Option<String> {
    get_secret(KEYCHAIN_GOOGLE_MAPS_API_KEY).ok().flatten()
}

pub fn set_google_maps_api_key(value: &str) -> Result<(), keyring::Error> {
    set_secret(KEYCHAIN_GOOGLE_MAPS_API_KEY, value)
}

pub fn delete_google_maps_api_key() -> Result<(), keyring::Error> {
    delete_secret(KEYCHAIN_GOOGLE_MAPS_API_KEY)
}

pub fn google_maps_api_key_saved() -> bool {
    google_maps_api_key_from_keychain().is_some()
}

pub fn set_umap_password(value: &str) -> Result<(), keyring::Error> {
    set_secret(KEYCHAIN_UMAP_PASSWORD, value)
}

pub fn delete_umap_password() -> Result<(), keyring::Error> {
    delete_secret(KEYCHAIN_UMAP_PASSWORD)
}

pub fn umap_password_saved() -> bool {
    get_secret(KEYCHAIN_UMAP_PASSWORD).ok().flatten().is_some()
}

pub fn umap_password_from_keychain() -> Option<String> {
    get_secret(KEYCHAIN_UMAP_PASSWORD).ok().flatten()
}

fn read_settings() -> Result<AppSettings, Box<dyn std::error::Error>> {
    let path = config_path()?;
    if !path.exists() {
        let settings = AppSettings::default();
        save_settings(&settings)?;
        return Ok(settings);
    }

    let text = fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}

fn config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let base = dirs::config_dir().ok_or("failed to resolve OS config directory")?;
    Ok(base.join(APP_CONFIG_DIR).join(CONFIG_FILE))
}

fn set_secret(key: &str, value: &str) -> Result<(), keyring::Error> {
    keyring::Entry::new(KEYCHAIN_SERVICE, key)?.set_password(value)
}

fn get_secret(key: &str) -> Result<Option<String>, keyring::Error> {
    match keyring::Entry::new(KEYCHAIN_SERVICE, key)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error),
    }
}

fn delete_secret(key: &str) -> Result<(), keyring::Error> {
    match keyring::Entry::new(KEYCHAIN_SERVICE, key)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_config_serializes_only_non_sensitive_values() {
        let settings = AppSettings {
            umap_default_url: "https://umap.example/en/".to_string(),
            umap_account: Some("alice".to_string()),
            locale: "zh-TW".to_string(),
            dev_mode: true,
        };

        let text = toml::to_string_pretty(&settings).unwrap();

        assert!(text.contains("umap_default_url"));
        assert!(text.contains("umap_account"));
        assert!(text.contains("locale"));
        assert!(text.contains("dev_mode"));
        assert!(!text.contains("password"));
        assert!(!text.contains("api_key"));
        assert!(!text.contains("secret"));
    }
}
