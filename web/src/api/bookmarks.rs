use axum::Json;
use std::collections::HashMap;

use axum::extract::Multipart;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use umap_core::google::{GooglePlace, extract_coords_from_url, parse_takeout};
use umap_core::google_maps_api::GooglePlaceDetails;
use umap_core::places_api::PlacesApiClient;
use umap_core::google_maps_api::{GoogleMapsClient, resolve_place_url_coords};

use crate::api::errors::ApiError;
use crate::session::AppSession;

#[derive(Serialize)]
pub struct BookmarkList {
    bookmarks: Vec<GooglePlace>,
    selected_ids: Vec<usize>,
}

#[derive(Serialize)]
pub struct ValidationInfo {
    total: usize,
    ready: usize,
    missing_coords: Vec<usize>,
    missing_name: Vec<usize>,
}

#[derive(Serialize)]
pub struct UploadResponse {
    bookmarks: Vec<GooglePlace>,
    selected_ids: Vec<usize>,
    validation: ValidationInfo,
}

fn compute_validation(places: &[GooglePlace]) -> ValidationInfo {
    let total = places.len();
    let mut missing_coords = Vec::new();
    let mut missing_name = Vec::new();
    let mut ready = 0usize;

    for (i, place) in places.iter().enumerate() {
        let has_coords = place.latitude.is_some() && place.longitude.is_some();
        let has_name = place.title.is_some() || place.place_name.is_some();
        if has_coords {
            ready += 1;
        } else {
            missing_coords.push(i);
        }
        if !has_name {
            missing_name.push(i);
        }
    }

    ValidationInfo {
        total,
        ready,
        missing_coords,
        missing_name,
    }
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

    let validation = compute_validation(&places);

    let mut app = AppSession::from_session(&session).await;
    app.bookmarks = Some(places.clone());
    app.selected_ids = None;
    app.save_to_session(&session).await;

    Ok(Json(UploadResponse {
        bookmarks: places,
        selected_ids: vec![],
        validation,
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
pub struct SelectRequest {
    selected_ids: Vec<usize>,
}

pub async fn select(
    session: Session,
    Json(req): Json<SelectRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut app = AppSession::from_session(&session).await;
    app.selected_ids = Some(req.selected_ids);
    app.save_to_session(&session).await;
    Ok(Json(serde_json::json!({ "ok": true })))
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

    // Store cookies for later auto-enrich
    app.google_cookies = Some(req.cookies.clone());

    let bookmarks = app
        .bookmarks
        .take()
        .ok_or_else(|| ApiError::BadRequest("No bookmarks in session. Upload first.".into()))?;

    let client = GoogleMapsClient::new(req.cookies);
    let mut enriched = 0usize;
    let mut skipped = 0usize;

    let mut updated: Vec<GooglePlace> = Vec::with_capacity(bookmarks.len());
    for mut place in bookmarks {
        // Try URL-based extraction first (always works, no cookies needed)
        if (place.latitude.is_none() || place.longitude.is_none())
            && let Some(url) = &place.url
            && let Some((lat, lng)) = extract_coords_from_url(url)
        {
            place.latitude = Some(lat.to_string());
            place.longitude = Some(lng.to_string());
            enriched += 1;
            updated.push(place);
            continue;
        }

        // If we have a place_id and still need coords, call Google Maps API
        if (place.latitude.is_none() || place.longitude.is_none())
            && let Some(pid) = &place.place_id
        {
            match client.get_place_details(pid).await {
                Ok(Some(details)) => {
                    apply_details_to_place(&mut place, &details);
                    enriched += 1;
                }
                Ok(None) => {
                    skipped += 1;
                }
                Err(e) => {
                    eprintln!("[enrich] API error for place_id={}: {}", pid, e);
                    skipped += 1;
                }
            }
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

pub async fn auto_enrich(
    session: Session,
) -> Result<impl IntoResponse, ApiError> {
    let mut app = AppSession::from_session(&session).await;
    let bookmarks = app
        .bookmarks
        .take()
        .ok_or_else(|| ApiError::BadRequest("No bookmarks in session. Upload first.".into()))?;

    let client = app.google_cookies.clone().map(GoogleMapsClient::new);
    let places_client = std::env::var("GOOGLE_MAP_API_KEY")
        .ok()
        .map(PlacesApiClient::new);

    let mut enriched = 0usize;
    let mut updated: Vec<GooglePlace> = Vec::with_capacity(bookmarks.len());
    for mut place in bookmarks {
        // Strategy 1: URL-based coordinate extraction (no API needed)
        if (place.latitude.is_none() || place.longitude.is_none())
            && let Some(url) = &place.url
            && let Some((lat, lng)) = extract_coords_from_url(url)
        {
            place.latitude = Some(lat.to_string());
            place.longitude = Some(lng.to_string());
            enriched += 1;
            updated.push(place);
            continue;
        }

        // Strategy 2: URL redirect resolution (no API needed)
        if (place.latitude.is_none() || place.longitude.is_none())
            && let Some(url) = &place.url
            && let Some((lat, lng)) = resolve_place_url_coords(url).await
        {
            place.latitude = Some(lat.to_string());
            place.longitude = Some(lng.to_string());
            enriched += 1;
            updated.push(place);
            continue;
        }

        // Strategy 3: Google Maps internal API (cookie-based)
        if (place.latitude.is_none() || place.longitude.is_none())
            && let Some(pid) = &place.place_id
            && let Some(ref c) = client
            && let Ok(Some(details)) = c.get_place_details(pid).await
        {
            apply_details_to_place(&mut place, &details);
            enriched += 1;
            updated.push(place);
            continue;
        }

        // Strategy 4: Official Places API (New) via Text Search (API key needed)
        if (place.latitude.is_none() || place.longitude.is_none())
            && let Some(ref p) = places_client
            && let Some(title) = place.title.as_deref().filter(|t| !t.is_empty())
        {
            let bias = match (
                place.latitude.as_ref().and_then(|s| s.parse::<f64>().ok()),
                place.longitude.as_ref().and_then(|s| s.parse::<f64>().ok()),
            ) {
                (Some(lat), Some(lng)) => Some((lat, lng)),
                _ => None,
            };
            match p.search_text(title, "zh-TW", bias).await {
                Ok(Some(api_place)) => {
                    if place.latitude.is_none() {
                        place.latitude = api_place.latitude.map(|v| v.to_string());
                    }
                    if place.longitude.is_none() {
                        place.longitude = api_place.longitude.map(|v| v.to_string());
                    }
                    if place.rating.is_none() {
                        place.rating = api_place.rating.map(|v| format!("{:.1}", v));
                    }
                    if place.website.is_none() {
                        place.website = api_place.website.clone();
                    }
                    if place.place_name.is_none() {
                        place.place_name = api_place.display_name.clone();
                    }
                    if place.original_name.is_none() {
                        place.original_name = api_place.display_name.clone();
                    }
                    if place.english_name.is_none()
                        && let Some(id) = &api_place.id
                    {
                        match p.get_place_details(id, "en").await {
                            Ok(en) => place.english_name = en.display_name,
                            Err(e) => eprintln!("[auto_enrich] english lookup failed: {e}"),
                        }
                    }
                    enriched += 1;
                }
                Err(e) => {
                    eprintln!("[auto_enrich] Places API error for '{title}': {e}");
                }
                Ok(None) => {}
            }
        }

        updated.push(place);
    }

    app.bookmarks = Some(updated.clone());
    app.save_to_session(&session).await;

    Ok(Json(serde_json::json!({
        "enriched": enriched,
        "bookmarks": updated,
    })))
}

/// Apply all available fields from `GooglePlaceDetails` to a `GooglePlace`,
/// only filling fields that are currently `None`.
fn apply_details_to_place(place: &mut GooglePlace, details: &GooglePlaceDetails) {
    if place.latitude.is_none() {
        place.latitude = details.latitude.map(|v| v.to_string());
    }
    if place.longitude.is_none() {
        place.longitude = details.longitude.map(|v| v.to_string());
    }
    if place.url.is_none() {
        place.url = details.url.clone();
    }
    if place.place_name.is_none() {
        place.place_name = details.place_name.clone();
    }
    if place.rating.is_none() {
        place.rating = details.rating.clone();
    }
    if place.website.is_none() {
        place.website = details.website.clone();
    }
    if place.description.is_none() {
        place.description = details.description.clone();
    }
    if place.original_name.is_none() {
        place.original_name = details.original_name.clone();
    }
    if place.english_name.is_none() {
        place.english_name = details.english_name.clone();
    }
}

