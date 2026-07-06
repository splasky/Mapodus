use axum::Json;
use std::collections::HashMap;

use axum::extract::Multipart;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use umap_core::google::{GooglePlace, extract_coords_from_url, parse_takeout};
use umap_core::google_maps_api::GoogleMapsClient;

use crate::api::errors::ApiError;
use crate::session::AppSession;

#[derive(Serialize)]
pub struct BookmarkList {
    bookmarks: Vec<GooglePlace>,
    selected_ids: Vec<usize>,
}

pub async fn upload(
    session: Session,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut csv_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
    {
        if field.name() == Some("file") {
            csv_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(e.to_string()))?
                    .to_vec(),
            );
        }
    }

    let data = csv_data.ok_or_else(|| ApiError::BadRequest("No file provided".into()))?;

    let tmp = std::env::temp_dir().join(format!("upload_{}.csv", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, &data).map_err(|e| ApiError::Internal(e.to_string()))?;

    let places = parse_takeout(tmp.to_str().unwrap())
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse CSV: {}", e)))?;

    let _ = std::fs::remove_file(&tmp);

    let mut app = AppSession::from_session(&session).await;
    app.bookmarks = Some(places.clone());
    app.selected_ids = None;
    app.save_to_session(&session).await;

    Ok(Json(BookmarkList {
        bookmarks: places,
        selected_ids: vec![],
    }))
}

pub async fn list(session: Session) -> Result<impl IntoResponse, ApiError> {
    let app = AppSession::from_session(&session).await;
    let bookmarks = app
        .bookmarks
        .clone()
        .ok_or_else(|| ApiError::BadRequest("No bookmarks uploaded".into()))?;
    let selected_ids = app.selected_ids.clone().unwrap_or_default();

    Ok(Json(BookmarkList {
        bookmarks,
        selected_ids,
    }))
}

#[derive(Deserialize)]
pub struct EnrichRequest {
    cookies: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct EnrichResponse {
    enriched: usize,
    skipped: usize,
    bookmarks: Vec<GooglePlace>,
}

pub async fn enrich(
    session: Session,
    Json(req): Json<EnrichRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut app = AppSession::from_session(&session).await;
    let bookmarks = app
        .bookmarks
        .take()
        .ok_or_else(|| ApiError::BadRequest("No bookmarks in session. Upload first.".into()))?;

    let client = GoogleMapsClient::new(req.cookies);
    let mut enriched = 0usize;
    let mut skipped = 0usize;

    let mut updated: Vec<GooglePlace> = Vec::with_capacity(bookmarks.len());
    for mut place in bookmarks {
        let mut changed = false;

        // Try URL-based coordinate extraction first; detail enrichment may still add other fields.
        if (place.latitude.is_none() || place.longitude.is_none())
            && let Some(url) = &place.url
            && let Some((lat, lng)) = extract_coords_from_url(url)
        {
            place.latitude = Some(lat.to_string());
            place.longitude = Some(lng.to_string());
            changed = true;
        }

        let needs_details = place.latitude.is_none()
            || place.longitude.is_none()
            || place.place_name.is_none()
            || place.rating.is_none()
            || place.website.is_none()
            || place.description.is_none()
            || place.english_name.is_none();

        if needs_details && let Some(pid) = &place.place_id {
            match client.get_place_details(pid).await {
                Ok(Some(details)) => {
                    if place.latitude.is_none() {
                        place.latitude = details.latitude.map(|v| v.to_string());
                    }
                    if place.longitude.is_none() {
                        place.longitude = details.longitude.map(|v| v.to_string());
                    }
                    if place.url.is_none() {
                        place.url = details.url;
                    }
                    if place.place_name.is_none() {
                        place.place_name = details.place_name;
                    }
                    if place.rating.is_none() {
                        place.rating = details.rating;
                    }
                    if place.website.is_none() {
                        place.website = details.website;
                    }
                    if place.description.is_none() {
                        place.description = details.description;
                    }
                    if place.english_name.is_none() {
                        place.english_name = details.english_name;
                    }
                    changed = true;
                }
                Ok(None) => {
                    eprintln!("[enrich] No details for place_id={}", pid);
                }
                Err(e) => {
                    eprintln!("[enrich] API error for place_id={}: {}", pid, e);
                }
            }
        }

        if changed {
            enriched += 1;
        } else {
            skipped += 1;
        }
        updated.push(place);
    }

    app.bookmarks = Some(updated.clone());
    app.save_to_session(&session).await;

    Ok(Json(EnrichResponse {
        enriched,
        skipped,
        bookmarks: updated,
    }))
}
