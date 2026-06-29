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
pub struct DebugRequest {
    cookies: HashMap<String, String>,
    list_id: Option<String>,
}

#[derive(Serialize)]
pub struct DebugResponse {
    mas_status: u16,
    mas_raw: String,
    mas_parsed_lists: serde_json::Value,
    list_debug: Option<ListDebugInfo>,
}

#[derive(Serialize)]
pub struct ListDebugInfo {
    list_status: u16,
    list_raw: String,
    list_parsed_places: serde_json::Value,
}

pub async fn debug_import(
    Json(req): Json<DebugRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let client = GoogleMapsClient::new(req.cookies);

    let (mas_status, mas_raw) = client
        .debug_mas()
        .await
        .map_err(|e| ApiError::Internal(format!("MAS debug error: {}", e)))?;

    // Try to parse the MAS response to show what structures were found
    let mas_parsed = if mas_status == 200 {
        let body = umap_core::google_maps_api::strip_xssi(&mas_raw)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .unwrap_or(serde_json::Value::Null);
        let root = body.as_array().cloned().unwrap_or_default();

        // Show all top-level elements that are arrays (potential list containers)
        let candidates: Vec<serde_json::Value> = root
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_array())
            .map(|(i, v)| {
                let arr = v.as_array().unwrap();
                let info = if arr.len() >= 4
                    && arr[0].is_null()
                    && arr[1].is_null()
                    && arr[2].is_null()
                {
                    format!("[null,null,null,...] {} entries in inner array",
                        arr[3].as_array().map(|a| a.len()).unwrap_or(0))
                } else {
                    format!("array of length {}", arr.len())
                };
                serde_json::json!({"index": i, "info": info})
            })
            .collect();

        serde_json::json!({
            "array_count": root.len(),
            "array_candidates": candidates,
            "top_level_types": root.iter().enumerate().map(|(i, v)| {
                serde_json::json!({"index": i, "type": match v {
                    serde_json::Value::Null => "null",
                    serde_json::Value::Array(_) => "array",
                    _ => "other",
                }})
            }).collect::<Vec<_>>(),
        })
    } else {
        serde_json::Value::Null
    };

    let list_debug = if let Some(list_id) = req.list_id {
        let (list_status, list_raw) = client
            .debug_getlist(&list_id)
            .await
            .map_err(|e| ApiError::Internal(format!("getlist debug error: {}", e)))?;

        let list_parsed = if list_status == 200 {
            let body = umap_core::google_maps_api::strip_xssi(&list_raw)
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .unwrap_or(serde_json::Value::Null);
            let root = body.as_array().and_then(|a| a.first()).and_then(|v| v.as_array());
            let entries = root.and_then(|r| r.get(8).and_then(|v| v.as_array()));
            serde_json::json!({
                "root_is_array": body.is_array(),
                "root_len": body.as_array().map(|a| a.len()),
                "first_elem_is_array": body.as_array().and_then(|a| a.first()).map(|v| v.is_array()),
                "first_elem_len": root.map(|r| r.len()),
                "entries_at_8": entries.map(|e| e.len()),
                "raw_first_500": &list_raw[..list_raw.len().min(500)],
            })
        } else {
            serde_json::Value::Null
        };

        Some(ListDebugInfo {
            list_status,
            list_raw: list_raw.clone(),
            list_parsed_places: list_parsed,
        })
    } else {
        None
    };

    Ok(Json(DebugResponse {
        mas_status,
        mas_raw: mas_raw.clone(),
        mas_parsed_lists: mas_parsed,
        list_debug,
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
