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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_all_fields_none() {
        let session = AppSession::default();
        assert!(session.bookmarks.is_none());
        assert!(session.selected_ids.is_none());
        assert!(session.umap_auth.is_none());
        assert!(session.umap_url.is_none());
        assert!(session.google_places.is_none());
        assert!(session.transfer_mode.is_none());
        assert!(session.session_google_maps_api_key.is_none());
        assert!(session.session_umap_password.is_none());
    }

    #[test]
    fn serialize_round_trip() {
        let session = AppSession {
            bookmarks: Some(vec![]),
            selected_ids: Some(vec![0, 1]),
            umap_auth: None,
            umap_url: Some("http://localhost:8000/".to_string()),
            google_places: None,
            transfer_mode: Some("single".to_string()),
            session_google_maps_api_key: None,
            session_umap_password: None,
        };
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: AppSession = serde_json::from_str(&json).unwrap();
        assert!(deserialized.bookmarks.is_some());
        assert_eq!(deserialized.selected_ids.unwrap(), vec![0, 1]);
        assert_eq!(deserialized.umap_url.unwrap(), "http://localhost:8000/");
        assert_eq!(deserialized.transfer_mode.unwrap(), "single");
    }
}
