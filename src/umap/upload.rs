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

    fn compute_center(fc: &geojson::FeatureCollection) -> Option<(f64, f64)> {
        for feature in &fc.features {
            if let Some(geometry) = &feature.geometry {
                if let geojson::Value::Point(coords) = &geometry.value {
                    if coords.len() >= 2 {
                        return Some((coords[0], coords[1]));
                    }
                }
            }
        }
        None
    }

    pub async fn create_map(
        &self,
        name: &str,
        fc: &geojson::FeatureCollection,
        auth: &CookieAuth,
    ) -> Result<String> {
        let create_url = format!("{}/map/create/", self.base_url);

        let (lon, lat) = Self::compute_center(fc).unwrap_or((0.0, 0.0));
        let center = serde_json::json!({"type": "Point", "coordinates": [lon, lat]}).to_string();
        let settings = serde_json::json!({
            "geometry": {"type": "Point", "coordinates": [lon, lat]},
            "properties": {
                "zoom": 12,
                "tilelayer": {
                    "url_template": "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
                    "attribution": "&copy; <a href=\"http://www.openstreetmap.org/copyright\">OpenStreetMap</a> contributors",
                    "maxZoom": 19,
                    "minZoom": 0
                }
            }
        }).to_string();

        let params = [
            ("name", name.to_string()),
            ("center", center),
            ("settings", settings),
        ];
        let response = self
            .client
            .post(&create_url)
            .headers(self.build_headers(auth))
            .form(&params)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to create map ({}): {}",
                status,
                body
            ));
        }

        let json_text = response.text().await?;
        let map_data: serde_json::Value = serde_json::from_str(&json_text)?;

        let map_id = map_data
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("No map id in create response: {}", json_text))?;

        // Get owner ID from response to preserve it in permissions update
        let owner_id = map_data
            .get("permissions")
            .and_then(|p| p.get("owner"))
            .and_then(|o| o.get("id"))
            .and_then(|v| v.as_u64());

        println!("Created map '{}' with ID: {}", name, map_id);

        // Set map to public (share_status=1) so it's visible without login
        if let Err(e) = self
            .set_map_permissions(&map_id.to_string(), owner_id, auth)
            .await
        {
            println!("Warning: Failed to set map permissions: {}", e);
        }

        Ok(map_id.to_string())
    }

    pub async fn set_map_permissions(
        &self,
        map_id: &str,
        owner_id: Option<u64>,
        auth: &CookieAuth,
    ) -> Result<()> {
        let url = format!("{}/map/{}/update/permissions/", self.base_url, map_id);
        let mut params: Vec<(&str, &str)> = vec![
            ("share_status", "1"),
            ("edit_status", "2"),
        ];
        let owner_str: String;
        if let Some(oid) = owner_id {
            owner_str = oid.to_string();
            params.push(("owner", &owner_str));
        }
        let response = self
            .client
            .post(&url)
            .headers(self.build_headers(auth))
            .form(&params)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to set map permissions ({}): {}",
                status,
                body
            ));
        }

        println!("Set map permissions: public (visible to everyone)");
        Ok(())
    }

    async fn find_existing_layer(
        &self,
        map_id: &str,
        layer_name: &str,
        auth: &CookieAuth,
    ) -> Result<Option<String>> {
        let url = format!("{}/map/{}/geojson/", self.base_url, map_id);
        let response = self
            .client
            .get(&url)
            .headers(self.build_headers(auth))
            .send()
            .await?;
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
                                return Ok(Some(id));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    fn generate_uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    pub async fn find_or_create_layer(
        &self,
        map_id: &str,
        layer_name: &str,
        auth: &CookieAuth,
    ) -> Result<String> {
        // Try to find existing layer
        if let Some(id) = self
            .find_existing_layer(map_id, layer_name, auth)
            .await?
        {
            println!("Found existing layer '{}' with ID: {}", layer_name, id);
            return Ok(id);
        }

        Err(anyhow::anyhow!(
            "No existing layer '{}' found on map {}. Use --layer-name with a layer that already exists, or skip this flag to use the default.",
            layer_name, map_id
        ))
    }

    pub async fn create_and_upload_layer(
        &self,
        map_id: &str,
        layer_name: &str,
        geojson: &geojson::FeatureCollection,
        auth: &CookieAuth,
    ) -> Result<String> {
        let layer_id = Self::generate_uuid();
        let url = format!(
            "{}/map/{}/datalayer/create/{}/",
            self.base_url, map_id, layer_id
        );

        let json_string = serde_json::to_string(geojson)?;
        let part = reqwest::multipart::Part::bytes(json_string.into_bytes())
            .file_name("data.geojson")
            .mime_str("application/json")?;

        let form = reqwest::multipart::Form::new()
            .part("geojson", part)
            .text("name", layer_name.to_string())
            .text("display_on_load", "true")
            .text("rank", "1");

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers(auth))
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Failed to create layer ({}): {}",
                status,
                body
            ));
        }

        let response_text = response.text().await?;
        let layer_data: serde_json::Value = serde_json::from_str(&response_text)?;

        let _id_check = layer_id_to_string(layer_data.get("id"))
            .ok_or_else(|| anyhow::anyhow!(
                "Failed to create layer, response: {}",
                response_text
            ))?;

        self.upload_geojson(map_id, &layer_id, layer_name, geojson, auth).await?;

        Ok(layer_id)
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
            .text("display_on_load", "true")
            .text("rank", "1");

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
