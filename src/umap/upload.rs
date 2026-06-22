// Copyright 2025 google-maps-to-umap Contributors
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

use super::auth::CookieAuth;
use anyhow::Result;

fn layer_id_to_string(id: Option<&serde_json::Value>) -> Option<String> {
    id.and_then(|v| {
        v.as_str()
            .map(|s| s.to_string())
            .or_else(|| v.as_i64().map(|n| n.to_string()))
            .or_else(|| v.as_f64().map(|n| n.to_string()))
    })
}

#[derive(Debug)]
pub struct UmapClient {
    base_url: String,
    client: reqwest::Client,
}

impl UmapClient {
    pub fn new(base_url: &str) -> Self {
        UmapClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("Failed to build reqwest client"),
        }
    }

    fn build_headers(&self, auth: &CookieAuth) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(cookie_val) = reqwest::header::HeaderValue::from_str(&auth.to_cookie_header()) {
            headers.insert(reqwest::header::COOKIE, cookie_val);
        }
        if let Ok(csrf_val) = reqwest::header::HeaderValue::from_str(&auth.to_csrf_header()) {
            headers.insert("X-CSRFToken", csrf_val);
        }
        headers
    }

    pub async fn find_or_create_layer(
        &self,
        map_id: &str,
        layer_name: &str,
        auth: &CookieAuth,
    ) -> Result<String> {
        let url = format!("{}/map/{}/geojson/", self.base_url, map_id);
        let response = self.client.get(&url).send().await?;
        let json_text = response.text().await?;
        let geojson_data: serde_json::Value = serde_json::from_str(&json_text)?;

        if let Some(datalayers) = geojson_data
            .get("properties")
            .and_then(|p| p.get("datalayers"))
        {
            if let Some(layers) = datalayers.as_array() {
                for layer in layers {
                    if let Some(name) = layer.get("name").and_then(|n| n.as_str()) {
                        if name == layer_name {
                            if let Some(id) = layer_id_to_string(layer.get("id")) {
                                return Ok(id);
                            }
                        }
                    }
                }
            }
        }

        // Layer not found, create new one
        let create_url = format!("{}/map/{}/datalayer/create/", self.base_url, map_id);
        let form = reqwest::multipart::Form::new()
            .text("name", layer_name.to_string())
            .text("display_on_load", "true".to_string());

        let response = self
            .client
            .post(&create_url)
            .headers(self.build_headers(auth))
            .multipart(form)
            .send()
            .await?;

        let response_text = response.text().await?;
        let layer_data: serde_json::Value = serde_json::from_str(&response_text)?;

        if let Some(id) = layer_id_to_string(layer_data.get("id")) {
            Ok(id)
        } else {
            Err(anyhow::anyhow!(
                "Failed to create layer, response: {}",
                response_text
            ))
        }
    }

    pub async fn upload_geojson(
        &self,
        map_id: &str,
        layer_id: &str,
        layer_name: &str,
        geojson: &geojson::FeatureCollection,
        auth: &CookieAuth,
    ) -> Result<()> {
        let json_string = serde_json::to_string(geojson)?;
        let url = format!(
            "{}/map/{}/datalayer/update/{}/",
            self.base_url, map_id, layer_id
        );

        let part = reqwest::multipart::Part::bytes(json_string.into_bytes())
            .file_name("data.geojson")
            .mime_str("application/json")?;

        let form = reqwest::multipart::Form::new()
            .part("geojson", part)
            .text("name", layer_name.to_string())
            .text("display_on_load", "true");

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers(auth))
            .multipart(form)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!(
                "Failed to upload geojson ({}): {}",
                status,
                body
            ))
        }
    }
}
