// Copyright 2026 HYChang
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
