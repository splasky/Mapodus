use axum::Json;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::api::errors::ApiError;
use crate::session::AppSession;
use crate::settings::{
    AppSettings, delete_google_maps_api_key, delete_umap_password, google_maps_api_key_saved,
    is_desktop_mode, load_settings, save_settings, set_google_maps_api_key, set_umap_password,
    umap_password_saved,
};

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    umap_default_url: String,
    umap_account: Option<String>,
    locale: String,
    dev_mode: bool,
    desktop_mode: bool,
    umap_password_saved: bool,
    google_maps_api_key_saved: bool,
}

#[derive(Debug, Deserialize)]
pub struct SettingsRequest {
    umap_default_url: String,
    umap_account: Option<String>,
    locale: String,
    dev_mode: bool,
    umap_password: Option<String>,
    clear_umap_password: bool,
    google_maps_api_key: Option<String>,
    clear_google_maps_api_key: bool,
}

pub async fn get(session: Session) -> Result<impl IntoResponse, ApiError> {
    let app = AppSession::from_session(&session).await;
    Ok(Json(settings_response(&app)))
}

pub async fn update(
    session: Session,
    Json(req): Json<SettingsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut app = AppSession::from_session(&session).await;
    let desktop_mode = is_desktop_mode();
    let current_settings = load_settings();
    let settings = AppSettings {
        umap_default_url: normalize_umap_url(
            &req.umap_default_url,
            &current_settings.umap_default_url,
        ),
        umap_account: normalize_optional(req.umap_account),
        locale: normalize_locale(&req.locale),
        dev_mode: req.dev_mode,
    };

    save_settings(&settings)
        .map_err(|error| ApiError::Internal(format!("Failed to save settings: {error}")))?;

    app.umap_url = Some(settings.umap_default_url.clone());

    update_secret(
        desktop_mode,
        req.umap_password,
        req.clear_umap_password,
        &mut app.session_umap_password,
        set_umap_password,
        delete_umap_password,
    )?;
    update_secret(
        desktop_mode,
        req.google_maps_api_key,
        req.clear_google_maps_api_key,
        &mut app.session_google_maps_api_key,
        set_google_maps_api_key,
        delete_google_maps_api_key,
    )?;

    app.save_to_session(&session).await;
    Ok(Json(settings_response(&app)))
}

fn settings_response(app: &AppSession) -> SettingsResponse {
    let settings = load_settings();
    let desktop_mode = is_desktop_mode();
    SettingsResponse {
        umap_default_url: settings.umap_default_url,
        umap_account: settings.umap_account,
        locale: settings.locale,
        dev_mode: settings.dev_mode,
        desktop_mode,
        umap_password_saved: if desktop_mode {
            umap_password_saved()
        } else {
            app.session_umap_password.is_some()
        },
        google_maps_api_key_saved: if desktop_mode {
            google_maps_api_key_saved()
        } else {
            app.session_google_maps_api_key.is_some()
        },
    }
}

fn update_secret(
    desktop_mode: bool,
    value: Option<String>,
    clear: bool,
    session_value: &mut Option<String>,
    set_desktop_secret: impl Fn(&str) -> Result<(), keyring::Error>,
    delete_desktop_secret: impl Fn() -> Result<(), keyring::Error>,
) -> Result<(), ApiError> {
    if clear {
        *session_value = None;
        if desktop_mode {
            delete_desktop_secret().map_err(|error| {
                ApiError::Internal(format!("Failed to delete secure credential: {error}"))
            })?;
        }
        return Ok(());
    }

    let Some(value) = normalize_optional(value) else {
        return Ok(());
    };

    if desktop_mode {
        set_desktop_secret(&value).map_err(|error| {
            ApiError::Internal(format!("Failed to save secure credential: {error}"))
        })?;
    } else {
        *session_value = Some(value);
    }
    Ok(())
}

fn normalize_umap_url(value: &str, default_url: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default_url.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_locale(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "en".to_string()
    } else {
        value.to_string()
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_umap_url_uses_existing_config_default() {
        let default_url = "https://umap.example/en/";

        assert_eq!(normalize_umap_url("   ", default_url), default_url);
    }
}
