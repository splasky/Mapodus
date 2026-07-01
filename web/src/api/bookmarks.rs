use axum::Json;
use axum::extract::Multipart;
use axum::response::IntoResponse;
use serde::Serialize;
use tower_sessions::Session;
use umap_core::google::{GooglePlace, parse_takeout};

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
