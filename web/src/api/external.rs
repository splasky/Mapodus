use axum::Json;
use axum::response::IntoResponse;
use reqwest::Url;
use serde::Deserialize;
use std::process::Command;

use crate::api::errors::ApiError;
use crate::settings::is_desktop_mode;

#[derive(Debug, Deserialize)]
pub struct OpenExternalRequest {
    url: String,
}

pub async fn open(Json(req): Json<OpenExternalRequest>) -> Result<impl IntoResponse, ApiError> {
    if !is_desktop_mode() {
        return Err(ApiError::BadRequest(
            "External opener is only available in desktop mode".into(),
        ));
    }

    let url = validate_external_url(&req.url)?;
    open_with_default_browser(url)?;
    Ok(Json(serde_json::json!({ "opened": true })))
}

fn validate_external_url(url: &str) -> Result<&str, ApiError> {
    let parsed = Url::parse(url).map_err(|_| ApiError::BadRequest("Invalid map URL".into()))?;
    match parsed.scheme() {
        "http" | "https" => Ok(url),
        _ => Err(ApiError::BadRequest(
            "Only HTTP and HTTPS map URLs can be opened".into(),
        )),
    }
}

fn open_with_default_browser(url: &str) -> Result<(), ApiError> {
    // Tauri 2 does not open target=_blank links from this externally served
    // webview reliably, so the embedded backend delegates to the OS opener.
    let mut command = default_browser_command(url);
    command
        .spawn()
        .map_err(|error| ApiError::Internal(format!("Failed to open map URL: {error}")))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn default_browser_command(url: &str) -> Command {
    let mut command = Command::new("open");
    command.arg(url);
    command
}

#[cfg(target_os = "windows")]
fn default_browser_command(url: &str) -> Command {
    let mut command = Command::new("rundll32");
    command.args(["url.dll,FileProtocolHandler", url]);
    command
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_browser_command(url: &str) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(url);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_http_and_https_urls() {
        assert!(validate_external_url("http://example.test/map").is_ok());
        assert!(validate_external_url("https://example.test/map").is_ok());
    }

    #[test]
    fn rejects_non_web_urls() {
        assert!(validate_external_url("file:///etc/passwd").is_err());
        assert!(validate_external_url("javascript:alert(1)").is_err());
        assert!(validate_external_url("not a url").is_err());
    }
}
