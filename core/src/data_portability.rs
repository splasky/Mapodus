use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::AppError;
use crate::google::GooglePlace;

pub const MAPS_STARRED_PLACES_RESOURCE: &str = "maps.starred_places";
pub const SAVED_COLLECTIONS_RESOURCE: &str = "saved.collections";

pub const MAPS_STARRED_PLACES_SCOPE: &str =
    "https://www.googleapis.com/auth/dataportability.maps.starred_places";
pub const SAVED_COLLECTIONS_SCOPE: &str =
    "https://www.googleapis.com/auth/dataportability.saved.collections";

pub const DEFAULT_SAVED_PLACES_RESOURCES: &[&str] =
    &[MAPS_STARRED_PLACES_RESOURCE, SAVED_COLLECTIONS_RESOURCE];

const DEFAULT_BASE_URL: &str = "https://dataportability.googleapis.com";

#[derive(Debug, Clone)]
pub struct DataPortabilityClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitiateArchiveRequest {
    pub resources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InitiateArchiveResponse {
    pub archive_job_id: String,
    pub access_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArchiveState {
    StateUnspecified,
    InProgress,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortabilityArchiveState {
    pub state: ArchiveState,
    #[serde(default)]
    pub urls: Vec<String>,
    pub name: Option<String>,
    pub start_time: Option<String>,
    pub export_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedArchiveFile {
    pub url: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedDataPortabilityPlaces {
    pub places: Vec<GooglePlace>,
    pub skipped_files: Vec<String>,
}

impl DataPortabilityClient {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    pub async fn initiate_saved_places_archive(
        &self,
        access_token: &str,
    ) -> Result<InitiateArchiveResponse, AppError> {
        self.initiate_archive(access_token, DEFAULT_SAVED_PLACES_RESOURCES, None, None)
            .await
    }

    pub async fn initiate_archive(
        &self,
        access_token: &str,
        resources: &[&str],
        start_time: Option<String>,
        end_time: Option<String>,
    ) -> Result<InitiateArchiveResponse, AppError> {
        if resources.is_empty() {
            return Err(AppError::Config(
                "at least one Data Portability resource is required".into(),
            ));
        }

        let request = InitiateArchiveRequest {
            resources: resources
                .iter()
                .map(|resource| resource.to_string())
                .collect(),
            start_time,
            end_time,
        };
        let url = format!("{}/v1/portabilityArchive:initiate", self.base_url);
        let response = self
            .client
            .post(url)
            .bearer_auth(access_token)
            .json(&request)
            .send()
            .await?;

        parse_json_response(response, "Data Portability archive initiate").await
    }

    pub async fn get_archive_state(
        &self,
        access_token: &str,
        archive_job_id: &str,
    ) -> Result<PortabilityArchiveState, AppError> {
        let archive_job_id = archive_job_id.trim_start_matches("archiveJobs/");
        let url = format!(
            "{}/v1/archiveJobs/{}/portabilityArchiveState",
            self.base_url, archive_job_id
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(access_token)
            .send()
            .await?;

        parse_json_response(response, "Data Portability archive state").await
    }

    pub async fn download_archive_files(
        &self,
        state: &PortabilityArchiveState,
    ) -> Result<Vec<DownloadedArchiveFile>, AppError> {
        if state.state != ArchiveState::Complete {
            return Err(AppError::Config(format!(
                "archive is not complete: {:?}",
                state.state
            )));
        }

        let mut files = Vec::new();
        for url in &state.urls {
            let response = self.client.get(url).send().await?;
            let status = response.status();
            let bytes = response.bytes().await?;
            if !status.is_success() {
                return Err(AppError::Http(format!(
                    "Data Portability archive download returned {} for {}",
                    status, url
                )));
            }
            files.push(DownloadedArchiveFile {
                url: url.clone(),
                bytes: bytes.to_vec(),
            });
        }

        Ok(files)
    }
}

impl Default for DataPortabilityClient {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_downloaded_archive_files(
    files: &[DownloadedArchiveFile],
) -> Result<ParsedDataPortabilityPlaces, AppError> {
    let mut parsed = ParsedDataPortabilityPlaces::default();

    for file in files {
        match parse_archive_file(&file.url, &file.bytes)? {
            Some(mut places) => parsed.places.append(&mut places),
            None => parsed.skipped_files.push(file.url.clone()),
        }
    }

    Ok(parsed)
}

pub fn parse_archive_file(
    file_name: &str,
    bytes: &[u8],
) -> Result<Option<Vec<GooglePlace>>, AppError> {
    let trimmed = trim_utf8_bom(bytes);
    let lower_name = file_name.to_lowercase();

    if lower_name.ends_with(".geojson")
        || lower_name.ends_with(".json")
        || trimmed.first().copied() == Some(b'{')
    {
        return parse_starred_places_geojson(trimmed).map(Some);
    }

    if lower_name.ends_with(".csv") || looks_like_csv(trimmed) {
        return parse_saved_collections_csv(trimmed).map(Some);
    }

    Ok(None)
}

pub fn parse_starred_places_geojson(bytes: &[u8]) -> Result<Vec<GooglePlace>, AppError> {
    let root: JsonValue = serde_json::from_slice(bytes)?;
    let features = root
        .get("features")
        .and_then(|value| value.as_array())
        .ok_or_else(|| AppError::Parse("Data Portability GeoJSON missing features".into()))?;

    Ok(features
        .iter()
        .filter_map(google_place_from_starred_feature)
        .collect())
}

pub fn parse_saved_collections_csv(bytes: &[u8]) -> Result<Vec<GooglePlace>, AppError> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader.headers().map_err(csv_error)?.clone();
    let header_map = headers
        .iter()
        .enumerate()
        .map(|(index, header)| (normalize_header(header), index))
        .collect::<Vec<_>>();
    let mut places = Vec::new();

    for record in reader.records() {
        let record = record.map_err(csv_error)?;
        if record.iter().all(|field| field.trim().is_empty()) {
            continue;
        }

        let title = field_by_any_header(&record, &header_map, &["title", "name"]);
        let url = field_by_any_header(&record, &header_map, &["url", "link"]);
        let notes = field_by_any_header(&record, &header_map, &["note", "notes"]);
        let comments = field_by_any_header(&record, &header_map, &["comment", "comments"]);
        let tags =
            field_by_any_header(&record, &header_map, &["collection", "collections", "list"]);

        places.push(GooglePlace {
            title: title.clone(),
            notes,
            url,
            tags,
            comments,
            latitude: None,
            longitude: None,
            place_name: title,
            rating: None,
            website: None,
            description: None,
            original_name: None,
            english_name: None,
            place_id: None,
        });
    }

    Ok(places)
}

async fn parse_json_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    context: &str,
) -> Result<T, AppError> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(AppError::Http(format!(
            "{} returned {}: {}",
            context, status, body
        )));
    }

    Ok(serde_json::from_str(&body)?)
}

fn google_place_from_starred_feature(feature: &JsonValue) -> Option<GooglePlace> {
    let properties = feature.get("properties")?;
    let location = first_location(properties.get("location"));
    let title = location
        .and_then(|value| value.get("name"))
        .and_then(|value| value.as_str())
        .or_else(|| properties.get("name").and_then(|value| value.as_str()))
        .map(|value| value.to_string());
    let address = location
        .and_then(|value| value.get("address"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let url = properties
        .get("google_maps_url")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let (latitude, longitude) = feature_coordinates(feature);

    Some(GooglePlace {
        title: title.clone(),
        notes: None,
        url,
        tags: Some("Google Data Portability: maps.starred_places".into()),
        comments: None,
        latitude: latitude.map(|value| value.to_string()),
        longitude: longitude.map(|value| value.to_string()),
        place_name: title,
        rating: None,
        website: None,
        description: address,
        original_name: None,
        english_name: None,
        place_id: None,
    })
}

fn first_location(location: Option<&JsonValue>) -> Option<&JsonValue> {
    match location? {
        JsonValue::Array(items) => items.first(),
        JsonValue::Object(_) => location,
        _ => None,
    }
}

fn feature_coordinates(feature: &JsonValue) -> (Option<f64>, Option<f64>) {
    let coordinates = feature
        .get("geometry")
        .and_then(|geometry| geometry.get("coordinates"))
        .and_then(|coordinates| coordinates.as_array());
    let Some(coordinates) = coordinates else {
        return (None, None);
    };

    let longitude = coordinates.first().and_then(|value| value.as_f64());
    let latitude = coordinates.get(1).and_then(|value| value.as_f64());

    match (latitude, longitude) {
        (Some(0.0), Some(0.0)) => (None, None),
        _ => (latitude, longitude),
    }
}

fn field_by_any_header(
    record: &csv::StringRecord,
    header_map: &[(String, usize)],
    candidates: &[&str],
) -> Option<String> {
    candidates.iter().find_map(|candidate| {
        header_map
            .iter()
            .find(|(header, _)| header == candidate || header.contains(candidate))
            .and_then(|(_, index)| record.get(*index))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn normalize_header(header: &str) -> String {
    header.trim().to_lowercase().replace([' ', '-', '_'], "")
}

fn trim_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes)
}

fn looks_like_csv(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|text| text.lines().next())
        .is_some_and(|line| line.contains(','))
}

fn csv_error(error: csv::Error) -> AppError {
    AppError::Parse(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_starred_places_geojson() {
        let bytes = br#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [121.5654, 25.0330]},
                    "properties": {
                        "google_maps_url": "https://www.google.com/maps/place/?q=place_id:abc",
                        "location": [
                            {
                                "name": "Taipei 101",
                                "address": "No. 7, Xinyi Road, Taipei"
                            }
                        ]
                    }
                }
            ]
        }"#;

        let places = parse_starred_places_geojson(bytes).unwrap();

        assert_eq!(places.len(), 1);
        assert_eq!(places[0].title.as_deref(), Some("Taipei 101"));
        assert_eq!(places[0].place_name.as_deref(), Some("Taipei 101"));
        assert_eq!(places[0].latitude.as_deref(), Some("25.033"));
        assert_eq!(places[0].longitude.as_deref(), Some("121.5654"));
        assert_eq!(
            places[0].description.as_deref(),
            Some("No. 7, Xinyi Road, Taipei")
        );
        assert_eq!(
            places[0].url.as_deref(),
            Some("https://www.google.com/maps/place/?q=place_id:abc")
        );
    }

    #[test]
    fn skips_zero_zero_starred_place_coordinates() {
        let bytes = br#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [0, 0]},
                    "properties": {"location": {"name": "Unknown"}}
                }
            ]
        }"#;

        let places = parse_starred_places_geojson(bytes).unwrap();

        assert_eq!(places[0].latitude, None);
        assert_eq!(places[0].longitude, None);
    }

    #[test]
    fn parses_saved_collections_csv() {
        let bytes = b"Title,URL,Note,Comments,Collection\nNight Market,\"https://maps.google.com/?q=25.0,121.0\",Try dinner,Busy,Food\n";

        let places = parse_saved_collections_csv(bytes).unwrap();

        assert_eq!(places.len(), 1);
        assert_eq!(places[0].title.as_deref(), Some("Night Market"));
        assert_eq!(
            places[0].url.as_deref(),
            Some("https://maps.google.com/?q=25.0,121.0")
        );
        assert_eq!(places[0].notes.as_deref(), Some("Try dinner"));
        assert_eq!(places[0].comments.as_deref(), Some("Busy"));
        assert_eq!(places[0].tags.as_deref(), Some("Food"));
    }

    #[test]
    fn parses_downloaded_mixed_archive_files() {
        let files = vec![
            DownloadedArchiveFile {
                url: "starred.geojson".into(),
                bytes: br#"{"type":"FeatureCollection","features":[]}"#.to_vec(),
            },
            DownloadedArchiveFile {
                url: "saved.csv".into(),
                bytes: b"Title,URL\nPlace,https://example.com\n".to_vec(),
            },
            DownloadedArchiveFile {
                url: "image.png".into(),
                bytes: vec![1, 2, 3],
            },
        ];

        let parsed = parse_downloaded_archive_files(&files).unwrap();

        assert_eq!(parsed.places.len(), 1);
        assert_eq!(parsed.skipped_files, vec!["image.png"]);
    }

    #[test]
    fn deserializes_archive_state_response() {
        let state: PortabilityArchiveState = serde_json::from_str(
            r#"{
                "state": "COMPLETE",
                "urls": ["https://storage.googleapis.com/archive"],
                "name": "archiveJobs/job-1/portabilityArchiveState"
            }"#,
        )
        .unwrap();

        assert_eq!(state.state, ArchiveState::Complete);
        assert_eq!(state.urls.len(), 1);
    }
}
