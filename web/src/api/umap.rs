use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use umap_core::convert::Converter;
use umap_core::umap::UmapClient;

use crate::api::errors::ApiError;
use crate::session::AppSession;

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
    map_id: String,
    map_url: String,
    message: String,
}

pub async fn connect(
    session: Session,
    Json(req): Json<ConnectRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let auth = umap_core::umap::login::proxy_login(&req.umap_url, &req.username, &req.password)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("uMap login failed: {}", e)))?;

    let mut app = AppSession::from_session(&session).await;
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
        umap_url: app.umap_url.clone(),
    }))
}

pub async fn transfer(
    session: Session,
    Json(req): Json<TransferRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let app = AppSession::from_session(&session).await;

    let auth = app.umap_auth.clone()
        .ok_or_else(|| ApiError::Unauthorized("Not connected to uMap".into()))?;
    let umap_url = app.umap_url.clone()
        .ok_or_else(|| ApiError::Unauthorized("uMap URL not configured".into()))?;
    let places = app.bookmarks.as_ref()
        .ok_or_else(|| ApiError::BadRequest("No bookmarks uploaded".into()))?;

    let selected: Vec<_> = req.selected_ids.iter()
        .filter_map(|&i| places.get(i))
        .cloned()
        .collect();

    if selected.is_empty() {
        return Err(ApiError::BadRequest("No bookmarks selected".into()));
    }

    let fc = Converter::to_umap_geojson(&selected);
    let client = UmapClient::new(&umap_url);

    let result = client
        .create_map(
            &format!("Google Maps Saved ({})", chrono::Local::now().format("%Y-%m-%d")),
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

    let map_url = format!("{}/map/{}_{}", umap_url.trim_end_matches('/'), result.slug, result.id);

    Ok(Json(TransferResponse {
        success: true,
        map_id: result.id,
        map_url,
        message: "Map created and bookmarks uploaded".into(),
    }))
}
