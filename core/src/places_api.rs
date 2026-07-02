use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::google::GooglePlace;

/// A single place result returned by the Google Places API (New).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlacesApiPlace {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub rating: Option<f64>,
    pub website: Option<String>,
}

/// Client for the official Google Places API (New): <https://places.googleapis.com/v1>.
///
/// Requires an API key with the `places.googleapis.com` service enabled on its
/// GCP project (distinct from Google Maps session cookies used elsewhere in
/// this crate for the internal/unofficial endpoints).
pub struct PlacesApiClient {
    client: reqwest::Client,
    api_key: String,
}

const FIELD_MASK: &str = "id,displayName,location,rating,websiteUri";
const SEARCH_FIELD_MASK: &str = "places.id,places.displayName,places.location,places.rating,places.websiteUri";

impl PlacesApiClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        PlacesApiClient {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
        }
    }

    /// Text Search: resolve a free-text query (optionally biased toward a
    /// location) to the best-matching place. Used because legacy CIDs
    /// extracted from Google Maps Takeout URLs (e.g. `0x...:0x...`) are not
    /// valid Place IDs for this API.
    pub async fn search_text(
        &self,
        query: &str,
        language_code: &str,
        location_bias: Option<(f64, f64)>,
    ) -> Result<Option<PlacesApiPlace>> {
        let url = "https://places.googleapis.com/v1/places:searchText";
        let mut body = serde_json::json!({
            "textQuery": query,
            "languageCode": language_code,
        });
        if let Some((lat, lng)) = location_bias {
            body["locationBias"] = serde_json::json!({
                "circle": {
                    "center": { "latitude": lat, "longitude": lng },
                    "radius": 2000.0
                }
            });
        }

        let response = self
            .client
            .post(url)
            .header("X-Goog-Api-Key", &self.api_key)
            .header("X-Goog-FieldMask", SEARCH_FIELD_MASK)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(anyhow!("Places searchText failed ({}): {}", status, text));
        }

        let json: serde_json::Value = serde_json::from_str(&text)?;
        let first = json
            .get("places")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first());

        Ok(first.map(parse_place_json))
    }

    /// Place Details: fetch a place by its canonical `ChIJ...` Place ID.
    pub async fn get_place_details(
        &self,
        place_id: &str,
        language_code: &str,
    ) -> Result<PlacesApiPlace> {
        let url = format!("https://places.googleapis.com/v1/places/{}", place_id);

        let response = self
            .client
            .get(&url)
            .header("X-Goog-Api-Key", &self.api_key)
            .header("X-Goog-FieldMask", FIELD_MASK)
            .query(&[("languageCode", language_code)])
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(anyhow!(
                "Places getPlaceDetails failed for '{}' ({}): {}",
                place_id,
                status,
                text
            ));
        }

        let json: serde_json::Value = serde_json::from_str(&text)?;
        Ok(parse_place_json(&json))
    }
}

fn parse_place_json(value: &serde_json::Value) -> PlacesApiPlace {
    PlacesApiPlace {
        id: value.get("id").and_then(|v| v.as_str()).map(String::from),
        display_name: value
            .get("displayName")
            .and_then(|d| d.get("text"))
            .and_then(|v| v.as_str())
            .map(String::from),
        latitude: value
            .get("location")
            .and_then(|l| l.get("latitude"))
            .and_then(|v| v.as_f64()),
        longitude: value
            .get("location")
            .and_then(|l| l.get("longitude"))
            .and_then(|v| v.as_f64()),
        rating: value.get("rating").and_then(|v| v.as_f64()),
        website: value
            .get("websiteUri")
            .and_then(|v| v.as_str())
            .map(String::from),
    }
}

/// Enrich a [`GooglePlace`] in place using the official Places API (New).
///
/// Resolves the place via Text Search (by title, biased to existing
/// coordinates if present), then fetches an English-language display name
/// via Place Details for the `english_name` field. Only fills fields that
/// are currently `None`.
pub async fn enrich_place(client: &PlacesApiClient, place: &mut GooglePlace) -> Result<bool> {
    let query = match &place.title {
        Some(t) if !t.is_empty() => t.clone(),
        _ => return Ok(false),
    };

    let bias = match (
        place.latitude.as_ref().and_then(|s| s.parse::<f64>().ok()),
        place.longitude.as_ref().and_then(|s| s.parse::<f64>().ok()),
    ) {
        (Some(lat), Some(lng)) => Some((lat, lng)),
        _ => None,
    };

    let primary = client.search_text(&query, "zh-TW", bias).await?;
    let Some(primary) = primary else {
        return Ok(false);
    };

    if place.latitude.is_none() {
        place.latitude = primary.latitude.map(|v| v.to_string());
    }
    if place.longitude.is_none() {
        place.longitude = primary.longitude.map(|v| v.to_string());
    }
    if place.rating.is_none() {
        place.rating = primary.rating.map(|v| v.to_string());
    }
    if place.website.is_none() {
        place.website = primary.website.clone();
    }
    if place.place_name.is_none() {
        place.place_name = primary.display_name.clone();
    }
    if place.original_name.is_none() {
        place.original_name = primary.display_name.clone();
    }

    if place.english_name.is_none()
        && let Some(id) = &primary.id
    {
        match client.get_place_details(id, "en").await {
            Ok(en) => place.english_name = en.display_name,
            Err(e) => eprintln!("[enrich_place] english lookup failed for id={id}: {e}"),
        }
    }

    Ok(true)
}
