use std::collections::HashMap;

use axum::Json;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use umap_core::google::GooglePlace;
use umap_core::google_maps_api::{GoogleMapsClient, GoogleSavedPlace};

use crate::api::errors::ApiError;
use crate::session::AppSession;

#[derive(Deserialize)]
pub struct DebugRequest {
    cookies: HashMap<String, String>,
    list_id: Option<String>,
}

#[derive(Serialize)]
pub struct DebugResponse {
    lists: serde_json::Value,
    list_places: Option<serde_json::Value>,
}

pub async fn debug_import(Json(req): Json<DebugRequest>) -> Result<impl IntoResponse, ApiError> {
    // This route is for parser inspection only. Responses may contain private
    // account data, so do not save its raw output as fixtures.
    let client = GoogleMapsClient::new(req.cookies);

    let lists = client
        .fetch_saved_lists()
        .await
        .map_err(|e| ApiError::Internal(format!("MAS error: {}", e)))?;

    let list_places = if let Some(list_id) = req.list_id {
        let places = client
            .fetch_list_places(&list_id, "debug")
            .await
            .map_err(|e| ApiError::Internal(format!("getlist error: {}", e)))?;
        Some(serde_json::to_value(places).unwrap_or_default())
    } else {
        None
    };

    Ok(Json(DebugResponse {
        lists: serde_json::to_value(lists).unwrap_or_default(),
        list_places,
    }))
}

#[derive(Deserialize)]
pub struct ImportRequest {
    cookies: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct ListGroup {
    name: String,
    count: usize,
    places: Vec<GoogleSavedPlace>,
}

#[derive(Serialize)]
pub struct ImportResponse {
    lists: Vec<ListGroup>,
    total: usize,
}

#[derive(Deserialize)]
pub struct ConfirmRequest {
    selected_lists: Vec<String>,
    transfer_mode: Option<String>,
}

#[derive(Serialize)]
pub struct ConfirmResponse {
    success: bool,
    count: usize,
    message: String,
}

pub async fn import(
    session: Session,
    Json(req): Json<ImportRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let client = GoogleMapsClient::new(req.cookies);

    let all_places = client
        .get_all_saved_places()
        .await
        .map_err(|e| ApiError::Internal(format!("Google API error: {}", e)))?;

    // The frontend lets the user choose lists by name. Keep the raw places in
    // session for the confirm step, and return grouped copies for display.
    let mut list_map: HashMap<String, Vec<GoogleSavedPlace>> = HashMap::new();
    for place in &all_places {
        list_map
            .entry(place.list.clone())
            .or_default()
            .push(place.clone());
    }

    let lists: Vec<ListGroup> = list_map
        .into_iter()
        .map(|(name, places)| ListGroup {
            count: places.len(),
            name,
            places,
        })
        .collect();

    let total = all_places.len();

    let mut app = AppSession::from_session(&session).await;
    app.google_places = Some(all_places);
    app.save_to_session(&session).await;

    Ok(Json(ImportResponse { lists, total }))
}

pub async fn confirm(
    session: Session,
    Json(req): Json<ConfirmRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut app = AppSession::from_session(&session).await;

    let google_places = app
        .google_places
        .take()
        .ok_or_else(|| ApiError::BadRequest("No Google places in session. Import first.".into()))?;

    // Import is a two-step flow: `/google/import` fetches everything into the
    // session, then `/google/confirm` narrows it to the selected lists.
    let selected: Vec<GoogleSavedPlace> = google_places
        .into_iter()
        .filter(|p| req.selected_lists.contains(&p.list))
        .collect();

    if selected.is_empty() {
        return Err(ApiError::BadRequest("No places selected".into()));
    }

    // Convert live Google API places into the same shape as Takeout bookmarks
    // so downstream selection, conversion, and upload code stays source-agnostic.
    let bookmarks: Vec<GooglePlace> = selected
        .iter()
        .map(|p| {
            let mut tags = vec![format!("list:{}", p.list)];
            tags.push("google_maps_api".to_string());
            GooglePlace {
                title: p.title.clone(),
                notes: p.notes.clone(),
                url: p.url.clone(),
                tags: Some(tags.join(", ")),
                comments: None,
                latitude: p.latitude.map(|v| v.to_string()),
                longitude: p.longitude.map(|v| v.to_string()),
                place_name: p.place_name.clone(),
                rating: p.rating.clone(),
                website: p.website.clone(),
                description: p.description.clone().or_else(|| p.address.clone()),
                original_name: p
                    .original_name
                    .clone()
                    .or_else(|| p.title.clone())
                    .or_else(|| p.place_name.clone()),
                english_name: p.english_name.clone(),
                place_id: p.place_id.clone(),
                google_place_details: None,
            }
        })
        .collect();

    let count = bookmarks.len();
    app.bookmarks = Some(bookmarks);
    app.transfer_mode = Some(req.transfer_mode.unwrap_or_else(|| "single".into()));
    app.selected_ids = None;
    app.save_to_session(&session).await;

    Ok(Json(ConfirmResponse {
        success: true,
        count,
        message: format!("Imported {} places from Google Maps", count),
    }))
}
