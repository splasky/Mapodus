use std::collections::BTreeMap;

use axum::Json;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use umap_core::convert::Converter;
use umap_core::google::GooglePlace;
use umap_core::places_api::{PlacesApiClient, PlacesApiEnrichmentStats};
use umap_core::umap::UmapClient;

use crate::api::errors::ApiError;
use crate::session::AppSession;
use crate::settings;

#[derive(Deserialize)]
pub struct ConnectRequest {
    umap_url: String,
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    success: bool,
    message: String,
}

#[derive(Serialize)]
pub struct UmapStatus {
    connected: bool,
    umap_url: Option<String>,
}

#[derive(Deserialize)]
pub struct TransferRequest {
    selected_ids: Vec<usize>,
}

#[derive(Serialize)]
pub struct TransferResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    map_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    map_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maps: Option<Vec<MapResult>>,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapResult {
    pub name: String,
    pub map_id: String,
    pub map_url: String,
}

pub async fn connect(
    session: Session,
    Json(req): Json<ConnectRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let app = AppSession::from_session(&session).await;
    let password = connect_password(&req.password, &app)
        .ok_or_else(|| ApiError::BadRequest("uMap password is required".into()))?;

    let auth = umap_core::umap::login::proxy_login(&req.umap_url, &req.username, &password)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("uMap login failed: {}", e)))?;

    let mut app = app;
    app.umap_auth = Some(auth);
    app.umap_url = Some(req.umap_url);
    app.save_to_session(&session).await;

    Ok(Json(ConnectResponse {
        success: true,
        message: "Connected to uMap".into(),
    }))
}

pub async fn status(session: Session) -> Result<impl IntoResponse, ApiError> {
    let app = AppSession::from_session(&session).await;
    Ok(Json(UmapStatus {
        connected: app.umap_auth.is_some(),
        umap_url: app.umap_url.clone().or_else(|| Some(default_umap_url())),
    }))
}

fn default_umap_url() -> String {
    settings::default_umap_url()
}

fn connect_password(request_password: &str, app: &AppSession) -> Option<String> {
    let request_password = request_password.trim();
    if !request_password.is_empty() {
        return Some(request_password.to_string());
    }

    if settings::is_desktop_mode() {
        settings::umap_password_from_keychain()
    } else {
        app.session_umap_password.clone()
    }
}

async fn create_and_upload_map(
    places: &[GooglePlace],
    map_name: &str,
    layer_name: &str,
    auth: &umap_core::umap::CookieAuth,
    umap_url: &str,
    places_api_stats: &Option<PlacesApiEnrichmentStats>,
) -> Result<MapResult, ApiError> {
    let fc = Converter::to_umap_geojson(places);
    ensure_has_features(fc.features.len(), map_name, places_api_stats)?;
    let client = UmapClient::new(umap_url);

    let result = client
        .create_map(map_name, &fc, auth)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create map '{}': {}", map_name, e)))?;

    client
        .create_and_upload_layer(&result.id, layer_name, &fc, auth)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to upload to '{}': {}", map_name, e)))?;

    let map_url = format!(
        "{}/map/{}_{}",
        umap_url.trim_end_matches('/'),
        result.slug,
        result.id
    );

    Ok(MapResult {
        name: map_name.to_string(),
        map_id: result.id,
        map_url,
    })
}

pub async fn transfer(
    session: Session,
    Json(req): Json<TransferRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let app = AppSession::from_session(&session).await;

    let auth = app
        .umap_auth
        .clone()
        .ok_or_else(|| ApiError::Unauthorized("Not connected to uMap".into()))?;
    let umap_url = app
        .umap_url
        .clone()
        .ok_or_else(|| ApiError::Unauthorized("uMap URL not configured".into()))?;
    let places = app
        .bookmarks
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("No bookmarks uploaded".into()))?;

    let mut selected: Vec<_> = req
        .selected_ids
        .iter()
        .filter_map(|&i| places.get(i))
        .cloned()
        .collect();

    if selected.is_empty() {
        return Err(ApiError::BadRequest("No bookmarks selected".into()));
    }

    let places_api_stats = enrich_with_places_api_if_configured(
        &mut selected,
        app.session_google_maps_api_key.clone(),
    )
    .await?;
    let mode = app.transfer_mode.as_deref().unwrap_or("single");

    if mode == "per_list" {
        // Group selected places by the list: tag
        let mut groups: BTreeMap<String, Vec<GooglePlace>> = BTreeMap::new();
        for place in &selected {
            let list_name = place
                .tags
                .as_ref()
                .and_then(|t| {
                    t.split(", ")
                        .find(|s| s.starts_with("list:"))
                        .map(|s| &s[5..])
                })
                .unwrap_or("Unknown")
                .to_string();
            groups.entry(list_name).or_default().push(place.clone());
        }

        if groups.is_empty() {
            return Err(ApiError::BadRequest("No grouped places found".into()));
        }

        let today = chrono::Local::now().format("%Y-%m-%d");
        let mut maps = Vec::new();
        for (name, group_places) in &groups {
            let mr = create_and_upload_map(
                group_places,
                &format!("{} ({})", name, today),
                name,
                &auth,
                &umap_url,
                &places_api_stats,
            )
            .await?;
            maps.push(mr);
        }

        Ok(Json(TransferResponse {
            success: true,
            map_id: None,
            map_url: None,
            maps: Some(maps),
            message: format!("Created {} maps from {} lists", groups.len(), groups.len()),
        }))
    } else {
        let fc = Converter::to_umap_geojson(&selected);
        ensure_has_features(fc.features.len(), "Google Maps Saved", &places_api_stats)?;
        let client = UmapClient::new(&umap_url);

        let result = client
            .create_map(
                &format!(
                    "Google Maps Saved ({})",
                    chrono::Local::now().format("%Y-%m-%d")
                ),
                &fc,
                &auth,
            )
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to create map: {}", e)))?;

        let layer_name = "Google Maps Saved";
        client
            .create_and_upload_layer(&result.id, layer_name, &fc, &auth)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to upload: {}", e)))?;

        let map_url = format!(
            "{}/map/{}_{}",
            umap_url.trim_end_matches('/'),
            result.slug,
            result.id
        );

        Ok(Json(TransferResponse {
            success: true,
            map_id: Some(result.id),
            map_url: Some(map_url),
            maps: None,
            message: "Map created and bookmarks uploaded".into(),
        }))
    }
}

async fn enrich_with_places_api_if_configured(
    places: &mut [GooglePlace],
    session_api_key: Option<String>,
) -> Result<Option<PlacesApiEnrichmentStats>, ApiError> {
    let client = PlacesApiClient::with_optional_api_key(places_api_key(session_api_key));
    client
        .enrich_places(places)
        .await
        .map(Some)
        .map_err(|e| ApiError::Internal(format!("Google Maps URI enrichment failed: {}", e)))
}

fn places_api_key(session_api_key: Option<String>) -> Option<String> {
    std::env::var("GOOGLE_MAPS_API_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty())
        .or_else(|| {
            if settings::is_desktop_mode() {
                settings::google_maps_api_key_from_keychain()
            } else {
                session_api_key
            }
        })
}

fn ensure_has_features(
    feature_count: usize,
    map_name: &str,
    places_api_stats: &Option<PlacesApiEnrichmentStats>,
) -> Result<(), ApiError> {
    if feature_count == 0 {
        let places_api_hint = match places_api_stats {
            Some(stats) => format!(
                " Google Maps URI enrichment ran but did not resolve usable coordinates (enriched: {}, skipped: {}, failed: {}).",
                stats.enriched, stats.skipped, stats.failed
            ),
            None => " Google Maps URI enrichment did not run.".to_string(),
        };
        return Err(ApiError::BadRequest(format!(
            "No data points were generated for '{}'. The selected POIs do not have usable coordinates.{}",
            map_name, places_api_hint
        )));
    }
    Ok(())
}
