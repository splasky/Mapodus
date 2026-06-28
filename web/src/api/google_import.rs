use std::collections::HashMap;

use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use umap_core::google::GooglePlace;
use umap_core::google_maps_api::{GoogleMapsClient, GoogleSavedPlace};

use crate::api::errors::ApiError;
use crate::session::AppSession;

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

    let selected: Vec<GoogleSavedPlace> = google_places
        .into_iter()
        .filter(|p| req.selected_lists.contains(&p.list))
        .collect();

    if selected.is_empty() {
        return Err(ApiError::BadRequest("No places selected".into()));
    }

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
                place_name: None,
                rating: None,
                website: None,
                description: p.address.clone(),
                original_name: None,
                english_name: None,
            }
        })
        .collect();

    let count = bookmarks.len();
    app.bookmarks = Some(bookmarks);
    app.selected_ids = None;
    app.save_to_session(&session).await;

    Ok(Json(ConfirmResponse {
        success: true,
        count,
        message: format!("Imported {} places from Google Maps", count),
    }))
}
