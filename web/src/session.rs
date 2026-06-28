use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use umap_core::google::GooglePlace;
use umap_core::umap::CookieAuth;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUser {
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSession {
    pub google_user: Option<GoogleUser>,
    pub bookmarks: Option<Vec<GooglePlace>>,
    pub selected_ids: Option<Vec<usize>>,
    pub umap_auth: Option<CookieAuth>,
    pub umap_url: Option<String>,
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
