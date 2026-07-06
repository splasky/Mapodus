use axum::Json;
use std::collections::HashMap;

use axum::extract::Multipart;
use axum::extract::Query;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use umap_core::google::{GooglePlace, extract_coords_from_url, parse_takeout};
use umap_core::google_maps_api::{
    GoogleMapsClient, GooglePlaceDetails, find_place_entry_in_array, resolve_place_url_coords,
};

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

    let mut places = parse_takeout(tmp.to_str().unwrap())
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse CSV: {}", e)))?;

    let _ = std::fs::remove_file(&tmp);

    // Resolve coordinates from URL redirects for places that have a URL but no coords
    for place in &mut places {
        if (place.latitude.is_none() || place.longitude.is_none())
            && let Some(url) = &place.url
            && let Some((lat, lng)) = resolve_place_url_coords(url).await
        {
            eprintln!(
                "[upload] Resolved coords from URL redirect: {:.6},{:.6}",
                lat, lng
            );
            place.latitude = Some(lat.to_string());
            place.longitude = Some(lng.to_string());
        }
    }

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

pub async fn auto_enrich(session: Session) -> Result<impl IntoResponse, ApiError> {
    let mut app = AppSession::from_session(&session).await;
    let bookmarks = app
        .bookmarks
        .take()
        .ok_or_else(|| ApiError::BadRequest("No bookmarks in session. Upload first.".into()))?;

    let client = app.google_cookies.clone().map(GoogleMapsClient::new);

    let mut enriched = 0usize;
    let mut updated: Vec<GooglePlace> = Vec::with_capacity(bookmarks.len());
    for mut place in bookmarks {
        // Try URL-based extraction (no cookies needed)
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

        // Try URL redirect resolution (no cookies needed)
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

        // If we have stored cookies and a place_id, call Google Maps API
        if (place.latitude.is_none() || place.longitude.is_none())
            && let Some(pid) = &place.place_id
            && let Some(ref c) = client
            && let Ok(Some(details)) = c.get_place_details(pid).await
        {
            if place.latitude.is_none() {
                place.latitude = details.latitude.map(|v| v.to_string());
            }
            if place.longitude.is_none() {
                place.longitude = details.longitude.map(|v| v.to_string());
            }
            if place.url.is_none() {
                place.url = details.url;
            }
            enriched += 1;
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

#[derive(Deserialize)]
pub struct DebugPlaceDetailsParams {
    place_id: String,
    cookies: String,
}

#[derive(Serialize)]
pub struct DebugPlaceDetailsResponse {
    raw_json: serde_json::Value,
    parsed: GooglePlaceDetails,
    entry: serde_json::Value,
}

pub async fn debug_place_details(
    Query(params): Query<DebugPlaceDetailsParams>,
) -> Result<impl IntoResponse, ApiError> {
    let cookie_map: HashMap<String, String> = params
        .cookies
        .split(';')
        .filter_map(|part| {
            let trimmed = part.trim();
            let eq = trimmed.find('=')?;
            Some((trimmed[..eq].to_string(), trimmed[eq + 1..].to_string()))
        })
        .collect();

    let client = GoogleMapsClient::new(cookie_map);
    let raw = client
        .debug_get_place_details(&params.place_id)
        .await?
        .unwrap_or(serde_json::Value::Null);

    let parsed = parse_place_details_direct(&raw, &params.place_id);
    let entry = find_place_entry_debug(&raw, &params.place_id);

    Ok(Json(DebugPlaceDetailsResponse {
        raw_json: raw,
        parsed,
        entry: entry.unwrap_or(serde_json::Value::Null),
    }))
}

fn parse_place_details_direct(value: &serde_json::Value, place_id: &str) -> GooglePlaceDetails {
    use umap_core::google_maps_api::GooglePlaceDetails;
    let root = match value.as_array() {
        Some(a) => a,
        None => {
            return GooglePlaceDetails {
                latitude: None,
                longitude: None,
                url: None,
                place_name: None,
                rating: None,
                website: None,
                description: None,
                original_name: None,
                english_name: None,
            };
        }
    };
    let entry = root.iter().find_map(|elem| {
        let arr = elem.as_array()?;
        find_place_entry_in_array(arr, place_id)
    });
    match entry {
        Some(entry_arr) => {
            let place_info = entry_arr.get(1).and_then(|v| v.as_array());
            let coords = place_info.and_then(|pi| pi.get(5).and_then(|v| v.as_array()));
            let latitude = coords.and_then(|c| c.get(2).and_then(|v| v.as_f64()));
            let longitude = coords.and_then(|c| c.get(3).and_then(|v| v.as_f64()));
            let pid = place_info.and_then(|pi| pi.get(7).and_then(|v| v.as_str()));
            let url = pid.map(|id| format!("https://www.google.com/maps/place/?q=place_id:{}", id));
            let place_name = entry_arr
                .get(2)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            GooglePlaceDetails {
                latitude,
                longitude,
                url,
                place_name,
                rating: None,
                website: None,
                description: None,
                original_name: None,
                english_name: None,
            }
        }
        None => GooglePlaceDetails {
            latitude: None,
            longitude: None,
            url: None,
            place_name: None,
            rating: None,
            website: None,
            description: None,
            original_name: None,
            english_name: None,
        },
    }
}

fn find_place_entry_debug(value: &serde_json::Value, place_id: &str) -> Option<serde_json::Value> {
    let root = value.as_array()?;
    root.iter().find_map(|elem| {
        let arr = elem.as_array()?;
        let _ = find_place_entry_in_array(arr, place_id)?;
        Some(elem.clone())
    })
}
