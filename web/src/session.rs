use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use umap_core::google::GooglePlace;
use umap_core::google_maps_api::GoogleSavedPlace;
use umap_core::umap::CookieAuth;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSession {
    pub bookmarks: Option<Vec<GooglePlace>>,
    pub selected_ids: Option<Vec<usize>>,
    pub umap_auth: Option<CookieAuth>,
    pub umap_url: Option<String>,
    pub google_places: Option<Vec<GoogleSavedPlace>>,
    pub transfer_mode: Option<String>,
    pub google_cookies: Option<HashMap<String, String>>,
    pub session_google_maps_api_key: Option<String>,
    pub session_umap_password: Option<String>,
}

impl AppSession {
    pub async fn from_session(session: &Session) -> Self {
        session
            .get::<Self>("app")
            .await
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    pub async fn save_to_session(&self, session: &Session) {
        let _ = session.insert("app", self).await;
    }
}
