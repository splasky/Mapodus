use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::error::AppError;
use crate::google::GooglePlace;

#[derive(Debug, Clone, Default)]
pub struct PlacesApiEnrichmentStats {
    pub enriched: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[derive(Debug, Clone)]
pub struct PlacesApiClient {
    client: reqwest::Client,
    api_key: Option<String>,
    locale: String,
}

impl PlacesApiClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_optional_api_key(Some(api_key.into()))
    }

    pub fn with_optional_api_key(api_key: Option<String>) -> Self {
        Self::with_optional_api_key_and_locale(api_key, "en")
    }

    pub fn with_optional_api_key_and_locale(
        api_key: Option<String>,
        locale: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.filter(|key| !key.trim().is_empty()),
            locale: normalize_locale(&locale.into()),
        }
    }

    pub async fn enrich_places(
        &self,
        places: &mut [GooglePlace],
    ) -> Result<PlacesApiEnrichmentStats, AppError> {
        let mut stats = PlacesApiEnrichmentStats::default();

        for place in places {
            if !needs_places_api_enrichment(place) {
                stats.skipped += 1;
                continue;
            }

            match self.resolve_place(place).await {
                Ok(Some(details)) => {
                    apply_places_api_details(place, details);
                    stats.enriched += 1;
                }
                Ok(None) => stats.skipped += 1,
                Err(error) => {
                    eprintln!(
                        "[places_api] Failed to enrich '{}': {}",
                        place
                            .title
                            .as_deref()
                            .or(place.place_name.as_deref())
                            .unwrap_or("unknown place"),
                        error
                    );
                    stats.failed += 1;
                }
            }
        }

        Ok(stats)
    }

    async fn resolve_place(
        &self,
        place: &GooglePlace,
    ) -> Result<Option<PlaceApiDetails>, AppError> {
        if let Some(place_id) = place
            .place_id
            .as_deref()
            .and_then(normalize_places_api_place_id)
        {
            match self.place_details(place_id).await {
                Ok(Some(details)) => return Ok(Some(details)),
                Ok(None) => {}
                Err(error) => {
                    eprintln!(
                        "[places_api] Place Details failed for place_id={}: {}. Falling back to Google Maps URI.",
                        place_id, error
                    );
                }
            }
        }

        for uri in google_maps_uris(place) {
            match self.resolve_google_maps_uri(&uri).await {
                Ok(Some(details)) => return Ok(Some(details)),
                Ok(None) => {}
                Err(error) => {
                    eprintln!(
                        "[places_api] Failed to resolve Google Maps URI '{}': {}",
                        uri, error
                    );
                }
            }
        }

        Ok(coordinate_query(place)
            .and_then(|query| parse_coordinates(&query).map(|coords| (query, coords)))
            .map(|(query, (latitude, longitude))| {
                PlaceApiDetails::from_coordinates(Some(query), latitude, longitude)
            }))
    }

    async fn resolve_google_maps_uri(
        &self,
        uri: &str,
    ) -> Result<Option<PlaceApiDetails>, AppError> {
        if let Some(place_id) = extract_places_api_place_id(uri) {
            match self.place_details(&place_id).await {
                Ok(Some(details)) => return Ok(Some(details)),
                Ok(None) => {}
                Err(error) => {
                    eprintln!(
                        "[places_api] Place Details failed for Google Maps URI place_id={}: {}. Falling back to URI Text Search.",
                        place_id, error
                    );
                }
            }
        }

        let is_legacy_feature_id = extract_legacy_feature_id(uri).is_some();
        let mut fallback_details = None;

        if let Some((latitude, longitude)) = extract_coordinates_from_google_maps_uri(uri) {
            fallback_details = Some(PlaceApiDetails::from_coordinates(None, latitude, longitude));
        }

        if fallback_details.is_none()
            && let Some((latitude, longitude)) = extract_feature_id_coordinates(uri)
        {
            fallback_details = Some(PlaceApiDetails::from_coordinates(None, latitude, longitude));
        }

        if let Some(mut details) = self
            .google_maps_uri_details(uri, !is_legacy_feature_id)
            .await?
        {
            if let Some(fallback) = fallback_details {
                details.apply_missing(fallback);
            }
            self.apply_uri_text_search_details(uri, &mut details).await;
            return Ok(Some(details));
        }

        if let Some(mut details) = fallback_details {
            self.apply_uri_text_search_details(uri, &mut details).await;
            return Ok(Some(details));
        }

        if is_legacy_feature_id {
            return Ok(None);
        }

        self.text_search(uri).await
    }

    async fn apply_uri_text_search_details(&self, uri: &str, details: &mut PlaceApiDetails) {
        if details.google_place_details.is_some() {
            return;
        }

        match self.text_search(uri).await {
            Ok(Some(text_search_details)) => {
                if details.can_merge_google_place_details(&text_search_details) {
                    details.apply_missing(text_search_details);
                }
            }
            Ok(None) => {}
            Err(error) => {
                eprintln!(
                    "[places_api] URI Text Search failed for Google Maps URI '{}': {}",
                    uri, error
                );
            }
        }
    }

    async fn google_maps_uri_details(
        &self,
        uri: &str,
        allow_html_coordinate_fallback: bool,
    ) -> Result<Option<PlaceApiDetails>, AppError> {
        let response = self
            .client
            .get(uri)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Safari/537.36",
            )
            .header(reqwest::header::ACCEPT_LANGUAGE, self.accept_language())
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(AppError::Http(format!(
                "Google Maps URI returned {}",
                status
            )));
        }

        if let Some(preview_uri) = extract_google_maps_preview_place_uri(&body)
            && let Some(details) = self.google_maps_preview_details(&preview_uri).await?
        {
            return Ok(Some(details));
        }

        if allow_html_coordinate_fallback {
            return Ok(extract_coordinates_from_google_maps_html(&body).map(
                |(latitude, longitude)| {
                    PlaceApiDetails::from_coordinates(None, latitude, longitude)
                },
            ));
        }

        Ok(None)
    }

    async fn google_maps_preview_details(
        &self,
        preview_uri: &str,
    ) -> Result<Option<PlaceApiDetails>, AppError> {
        let response = self
            .client
            .get(preview_uri)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Safari/537.36",
            )
            .header(reqwest::header::ACCEPT_LANGUAGE, self.accept_language())
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(AppError::Http(format!(
                "Google Maps preview returned {}",
                status
            )));
        }

        let body = strip_xssi(&body);
        let value: JsonValue = serde_json::from_str(body)?;
        Ok(PlaceApiDetails::from_google_maps_preview(&value))
    }

    async fn place_details(&self, place_id: &str) -> Result<Option<PlaceApiDetails>, AppError> {
        let Some(api_key) = self.api_key.as_deref() else {
            return Ok(None);
        };

        let url = format!("https://places.googleapis.com/v1/places/{}", place_id);
        let response = self
            .client
            .get(url)
            .query(&[("key", api_key), ("languageCode", self.locale.as_str())])
            .header("X-Goog-FieldMask", PLACE_DETAILS_FIELD_MASK)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(AppError::Http(format!(
                "Places Details returned {}: {}",
                status, body
            )));
        }

        let place: JsonValue = serde_json::from_str(&body)?;
        Ok(PlaceApiDetails::from_place_value(place))
    }

    async fn text_search(&self, query: &str) -> Result<Option<PlaceApiDetails>, AppError> {
        let Some(api_key) = self.api_key.as_deref() else {
            return Ok(None);
        };

        let response = self
            .client
            .post("https://places.googleapis.com/v1/places:searchText")
            .query(&[("key", api_key)])
            .header("X-Goog-FieldMask", TEXT_SEARCH_FIELD_MASK)
            .json(&serde_json::json!({
                "textQuery": query,
                "languageCode": self.locale.as_str(),
            }))
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(AppError::Http(format!(
                "Places Text Search returned {}: {}",
                status, body
            )));
        }

        let search: TextSearchResponse = serde_json::from_str(&body)?;
        Ok(search
            .places
            .into_iter()
            .find_map(PlaceApiDetails::from_place_value))
    }

    fn accept_language(&self) -> String {
        accept_language(&self.locale)
    }
}

fn normalize_locale(locale: &str) -> String {
    let trimmed = locale.trim();
    if trimmed.is_empty() {
        return "en".to_string();
    }

    if trimmed.eq_ignore_ascii_case("zh-tw") || trimmed.eq_ignore_ascii_case("zh-hant-tw") {
        return "zh-TW".to_string();
    }

    if trimmed.to_ascii_lowercase().starts_with("en") {
        return "en".to_string();
    }

    trimmed.to_string()
}

fn accept_language(locale: &str) -> String {
    match normalize_locale(locale).as_str() {
        "zh-TW" => "zh-TW,zh;q=0.9,en;q=0.8".to_string(),
        "en" => "en,zh-TW;q=0.6".to_string(),
        other => format!("{other},en;q=0.8"),
    }
}

const PLACE_DETAILS_FIELD_MASK: &str = concat!(
    "attributions,id,movedPlace,movedPlaceId,name,",
    "addressComponents,addressDescriptor,adrFormatAddress,formattedAddress,location,plusCode,postalAddress,shortFormattedAddress,types,viewport,",
    "accessibilityOptions,businessStatus,containingPlaces,displayName,googleMapsLinks,googleMapsUri,iconBackgroundColor,iconMaskBaseUri,openingDate,primaryType,primaryTypeDisplayName,pureServiceAreaBusiness,subDestinations,timeZone,utcOffsetMinutes"
);
const TEXT_SEARCH_FIELD_MASK: &str = concat!(
    "places.attributions,places.id,places.movedPlace,places.movedPlaceId,places.name,",
    "places.addressComponents,places.addressDescriptor,places.adrFormatAddress,places.formattedAddress,places.location,places.plusCode,places.postalAddress,places.shortFormattedAddress,places.types,places.viewport,",
    "places.accessibilityOptions,places.businessStatus,places.containingPlaces,places.displayName,places.googleMapsLinks,places.googleMapsUri,places.iconBackgroundColor,places.iconMaskBaseUri,places.openingDate,places.primaryType,places.primaryTypeDisplayName,places.pureServiceAreaBusiness,places.subDestinations,places.timeZone,places.utcOffsetMinutes"
);

#[derive(Debug, Clone)]
struct PlaceApiDetails {
    id: Option<String>,
    display_name: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    rating: Option<String>,
    website: Option<String>,
    google_maps_url: Option<String>,
    description: Option<String>,
    google_place_details: Option<JsonValue>,
}

impl PlaceApiDetails {
    fn from_place_value(value: JsonValue) -> Option<Self> {
        let place: PlacesApiPlace = serde_json::from_value(value.clone()).ok()?;
        Some(Self {
            id: place.id,
            display_name: place.display_name.and_then(|name| name.text),
            latitude: place.location.as_ref().map(|location| location.latitude),
            longitude: place.location.as_ref().map(|location| location.longitude),
            rating: None,
            website: None,
            google_maps_url: place.google_maps_uri,
            description: None,
            google_place_details: Some(value),
        })
    }

    fn from_coordinates(display_name: Option<String>, latitude: f64, longitude: f64) -> Self {
        Self {
            id: None,
            display_name,
            latitude: Some(latitude),
            longitude: Some(longitude),
            rating: None,
            website: None,
            google_maps_url: Some(format!(
                "https://www.google.com/maps/search/{},{}",
                latitude, longitude
            )),
            description: None,
            google_place_details: None,
        }
    }

    fn from_google_maps_preview(value: &JsonValue) -> Option<Self> {
        let place = value.get(6)?;
        let coords = json_path(value, &[4, 0]).and_then(|v| {
            let longitude = v.get(1).and_then(JsonValue::as_f64)?;
            let latitude = v.get(2).and_then(JsonValue::as_f64)?;
            valid_lat_lng(latitude, longitude).then_some((latitude, longitude))
        });

        Some(Self {
            id: None,
            display_name: json_path(place, &[11])
                .and_then(JsonValue::as_str)
                .map(str::to_string),
            latitude: coords.map(|(latitude, _)| latitude),
            longitude: coords.map(|(_, longitude)| longitude),
            rating: json_path(place, &[4, 7])
                .and_then(JsonValue::as_f64)
                .map(format_rating),
            website: json_path(place, &[7, 0])
                .and_then(JsonValue::as_str)
                .and_then(extract_target_url)
                .or_else(|| {
                    json_path(place, &[96, 10, 1, 0, 5, 2, 1])
                        .and_then(JsonValue::as_str)
                        .map(str::to_string)
                }),
            google_maps_url: json_path(place, &[42])
                .and_then(JsonValue::as_str)
                .map(str::to_string),
            description: extract_google_maps_preview_description(place),
            google_place_details: None,
        })
        .filter(|details| details.has_any_data())
    }

    fn has_any_data(&self) -> bool {
        self.display_name.is_some()
            || self.latitude.is_some()
            || self.longitude.is_some()
            || self.rating.is_some()
            || self.website.is_some()
            || self.google_maps_url.is_some()
            || self.description.is_some()
    }

    fn apply_missing(&mut self, fallback: Self) {
        if self.id.is_none() {
            self.id = fallback.id;
        }
        if self.display_name.is_none() {
            self.display_name = fallback.display_name;
        }
        if self.latitude.is_none() {
            self.latitude = fallback.latitude;
        }
        if self.longitude.is_none() {
            self.longitude = fallback.longitude;
        }
        if self.rating.is_none() {
            self.rating = fallback.rating;
        }
        if self.website.is_none() {
            self.website = fallback.website;
        }
        if self.google_maps_url.is_none() {
            self.google_maps_url = fallback.google_maps_url;
        }
        if self.description.is_none() {
            self.description = fallback.description;
        }
        if self.google_place_details.is_none() {
            self.google_place_details = fallback.google_place_details;
        }
    }

    fn can_merge_google_place_details(&self, details: &Self) -> bool {
        match (
            self.latitude,
            self.longitude,
            details.latitude,
            details.longitude,
        ) {
            (
                Some(existing_latitude),
                Some(existing_longitude),
                Some(details_latitude),
                Some(details_longitude),
            ) => {
                (existing_latitude - details_latitude).abs() <= 0.001
                    && (existing_longitude - details_longitude).abs() <= 0.001
            }
            _ => true,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlacesApiPlace {
    id: Option<String>,
    display_name: Option<LocalizedText>,
    location: Option<PlacesApiLocation>,
    google_maps_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TextSearchResponse {
    #[serde(default)]
    places: Vec<JsonValue>,
}

#[derive(Debug, Deserialize)]
struct PlacesApiLocation {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
struct LocalizedText {
    text: Option<String>,
}

fn needs_places_api_enrichment(place: &GooglePlace) -> bool {
    place.latitude.is_none()
        || place.longitude.is_none()
        || place.place_name.is_none()
        || place.url.is_none()
}

fn apply_places_api_details(place: &mut GooglePlace, details: PlaceApiDetails) {
    if place.original_name.is_none() {
        place.original_name = place.title.clone().or_else(|| place.place_name.clone());
    }
    if place.latitude.is_none() {
        place.latitude = details.latitude.map(|v| v.to_string());
    }
    if place.longitude.is_none() {
        place.longitude = details.longitude.map(|v| v.to_string());
    }
    if place.place_id.is_none() {
        place.place_id = details.id;
    }
    if place.place_name.is_none() {
        place.place_name = details.display_name.clone();
    }
    if place.rating.is_none() {
        place.rating = details.rating;
    }
    if place.website.is_none() {
        place.website = details.website;
    }
    if place.url.is_none() {
        place.url = details.google_maps_url;
    }
    if place.description.is_none() {
        place.description = details.description;
    }
    if place.google_place_details.is_none() {
        place.google_place_details = details.google_place_details;
    }
}

fn google_maps_uris(place: &GooglePlace) -> Vec<String> {
    [&place.url, &place.notes, &place.comments]
        .into_iter()
        .filter_map(|value| value.as_deref())
        .flat_map(extract_google_maps_uris)
        .collect()
}

fn extract_google_maps_preview_place_uri(html: &str) -> Option<String> {
    let marker = "href=\"/maps/preview/place?";
    let start = html.find(marker)? + "href=\"".len();
    let href = html[start..].split('"').next()?;
    let decoded = decode_html_entities(href);
    Some(format!("https://www.google.com{}", decoded))
}

fn decode_html_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn strip_xssi(input: &str) -> &str {
    input
        .strip_prefix(")]}'\n")
        .or_else(|| input.strip_prefix(")]}'"))
        .unwrap_or(input)
}

fn json_path<'a>(value: &'a JsonValue, path: &[usize]) -> Option<&'a JsonValue> {
    path.iter().try_fold(value, |current, index| {
        current.as_array().and_then(|array| array.get(*index))
    })
}

fn json_path_string(value: &JsonValue, path: &[usize]) -> Option<String> {
    json_path(value, path)
        .and_then(JsonValue::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
}

fn extract_target_url(input: &str) -> Option<String> {
    if input.starts_with("http://") || input.starts_with("https://") {
        return Some(input.to_string());
    }

    let url = reqwest::Url::parse(&format!("https://www.google.com{}", input)).ok()?;
    url.query_pairs()
        .find(|(key, _)| key == "q" || key == "url")
        .map(|(_, value)| value.into_owned())
}

fn extract_google_maps_preview_description(place: &JsonValue) -> Option<String> {
    [
        &[175, 9, 0, 0, 7, 0, 2, 15, 0, 0][..],
        &[32, 1, 1][..],
        &[32, 0, 1][..],
    ]
    .into_iter()
    .find_map(|path| json_path_string(place, path))
}

fn extract_google_maps_uris(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|part| part.trim_matches(|c: char| c == ',' || c == ')' || c == ']' || c == '}'))
        .filter(|part| part.contains("google.com/maps") || part.contains("maps.app.goo.gl"))
        .map(|part| part.to_string())
        .collect()
}

fn coordinate_query(place: &GooglePlace) -> Option<String> {
    place
        .title
        .as_deref()
        .or(place.place_name.as_deref())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
}

fn extract_places_api_place_id(input: &str) -> Option<String> {
    extract_place_id_from_query(input)
        .or_else(|| extract_place_id_after(input, "place_id:"))
        .or_else(|| extract_place_id_after(input, "places/"))
        .or_else(|| extract_place_id_after(input, "!1s"))
        .and_then(|id| normalize_places_api_place_id(&id).map(str::to_string))
}

fn extract_feature_id_coordinates(input: &str) -> Option<(f64, f64)> {
    let feature_id = extract_legacy_feature_id(input)?;
    let cell_token = feature_id.strip_prefix("0x")?.split(':').next()?.trim();
    let cell_id = s2::cellid::CellID::from_token(cell_token);
    if !cell_id.is_valid() {
        return None;
    }
    let lat_lng = s2::latlng::LatLng::from(cell_id);
    let latitude = lat_lng.lat.deg();
    let longitude = lat_lng.lng.deg();
    valid_lat_lng(latitude, longitude).then_some((latitude, longitude))
}

fn extract_legacy_feature_id(input: &str) -> Option<String> {
    let feature_id = extract_place_id_after(input, "!1s")?;
    (feature_id.starts_with("0x") && feature_id.contains(':')).then_some(feature_id)
}

fn extract_place_id_from_query(input: &str) -> Option<String> {
    let url = reqwest::Url::parse(input).ok()?;
    for (key, value) in url.query_pairs() {
        if matches!(key.as_ref(), "place_id" | "query_place_id") {
            return Some(value.into_owned());
        }
        if matches!(key.as_ref(), "q" | "query")
            && let Some(place_id) = value.strip_prefix("place_id:")
        {
            return Some(place_id.to_string());
        }
    }
    None
}

fn extract_place_id_after(input: &str, marker: &str) -> Option<String> {
    let start = input.find(marker)? + marker.len();
    let id = input[start..]
        .split(|c: char| {
            c == '!'
                || c == '/'
                || c == '?'
                || c == '&'
                || c == '#'
                || c == '"'
                || c == '\''
                || c.is_whitespace()
        })
        .next()?
        .trim();
    (!id.is_empty()).then(|| id.to_string())
}

fn normalize_places_api_place_id(input: &str) -> Option<&str> {
    let trimmed = input.trim().trim_start_matches("places/").trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with("0x")
        || trimmed.contains(':')
    {
        return None;
    }
    Some(trimmed)
}

fn extract_coordinates_from_google_maps_uri(input: &str) -> Option<(f64, f64)> {
    extract_coordinates_after(input, '@')
        .or_else(|| extract_coordinates_from_query(input))
        .or_else(|| extract_coordinates_from_data_params(input))
}

fn extract_coordinates_from_google_maps_html(input: &str) -> Option<(f64, f64)> {
    extract_staticmap_center(input).or_else(|| extract_preview_place_pb_coordinates(input))
}

fn extract_staticmap_center(input: &str) -> Option<(f64, f64)> {
    let marker = "staticmap?center=";
    let start = input.find(marker)? + marker.len();
    let encoded = input[start..]
        .split(['&', '"', '\'', '<'])
        .next()?
        .replace("%2C", ",")
        .replace("%2c", ",");
    parse_coordinate_pair(&encoded)
}

fn extract_preview_place_pb_coordinates(input: &str) -> Option<(f64, f64)> {
    // Google Maps preview pb uses !2d<lng>!3d<lat> for the focused place.
    let longitude = extract_number_after(input, "!2d")?;
    let latitude = extract_number_after(input, "!3d")?;
    valid_lat_lng(latitude, longitude).then_some((latitude, longitude))
}

fn extract_coordinates_after(input: &str, marker: char) -> Option<(f64, f64)> {
    let start = input.find(marker)? + marker.len_utf8();
    let candidate = input[start..].split(['/', '?', '&', '#']).next()?;
    parse_coordinate_pair(candidate)
}

fn extract_coordinates_from_query(input: &str) -> Option<(f64, f64)> {
    let url = reqwest::Url::parse(input).ok()?;
    for (key, value) in url.query_pairs() {
        if matches!(key.as_ref(), "q" | "query" | "ll" | "center")
            && let Some(coords) = parse_coordinates(&value)
        {
            return Some(coords);
        }
    }
    None
}

fn extract_coordinates_from_data_params(input: &str) -> Option<(f64, f64)> {
    let latitude = extract_number_after(input, "!3d")?;
    let longitude = extract_number_after(input, "!4d")?;
    valid_lat_lng(latitude, longitude).then_some((latitude, longitude))
}

fn extract_number_after(input: &str, marker: &str) -> Option<f64> {
    let start = input.find(marker)? + marker.len();
    input[start..]
        .split(|c: char| {
            c == '!'
                || c == '/'
                || c == '?'
                || c == '&'
                || c == '#'
                || c == ','
                || c.is_whitespace()
        })
        .next()?
        .parse()
        .ok()
}

fn parse_coordinates(input: &str) -> Option<(f64, f64)> {
    parse_coordinate_pair(input).or_else(|| parse_dms_coordinates(input))
}

fn parse_coordinate_pair(input: &str) -> Option<(f64, f64)> {
    let normalized = input
        .trim()
        .trim_matches(|c: char| c == '(' || c == ')' || c.is_whitespace())
        .replace(',', " ");
    let parts = normalized
        .split_whitespace()
        .filter_map(|part| part.parse::<f64>().ok())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    valid_lat_lng(parts[0], parts[1]).then_some((parts[0], parts[1]))
}

fn parse_dms_coordinates(input: &str) -> Option<(f64, f64)> {
    let compact = input.split_whitespace().collect::<String>();
    let lat_marker = compact
        .char_indices()
        .find(|(_, c)| matches!(c, 'N' | 'S' | 'n' | 's'))?;
    let lng_marker = compact[lat_marker.0 + lat_marker.1.len_utf8()..]
        .char_indices()
        .find(|(_, c)| matches!(c, 'E' | 'W' | 'e' | 'w'))?;
    let lat_end = lat_marker.0 + lat_marker.1.len_utf8();
    let lng_end = lat_end + lng_marker.0 + lng_marker.1.len_utf8();

    let lat = parse_dms_part(&compact[..lat_end])?;
    let lng = parse_dms_part(&compact[lat_end..lng_end])?;
    valid_lat_lng(lat, lng).then_some((lat, lng))
}

fn parse_dms_part(part: &str) -> Option<f64> {
    let direction = part
        .chars()
        .find(|c| matches!(c, 'N' | 'S' | 'E' | 'W' | 'n' | 's' | 'e' | 'w'))?;
    let sign = if matches!(direction, 'S' | 'W' | 's' | 'w') {
        -1.0
    } else {
        1.0
    };
    let numbers = part
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect::<Vec<_>>();
    if numbers.is_empty() {
        return None;
    }
    let degrees = numbers[0];
    let minutes = numbers.get(1).copied().unwrap_or(0.0);
    let seconds = numbers.get(2).copied().unwrap_or(0.0);
    Some(sign * (degrees + minutes / 60.0 + seconds / 3600.0))
}

fn valid_lat_lng(latitude: f64, longitude: f64) -> bool {
    latitude.is_finite()
        && longitude.is_finite()
        && (-90.0..=90.0).contains(&latitude)
        && (-180.0..=180.0).contains(&longitude)
}

fn format_rating(rating: f64) -> String {
    let rounded = (rating * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{:.0}", rounded)
    } else {
        format!("{:.1}", rounded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_html_entities_replaces_all_entities() {
        assert_eq!(
            decode_html_entities("&amp; &quot; &#39; &lt; &gt;"),
            "& \" ' < >"
        );
    }

    #[test]
    fn decode_html_entities_passes_through_plain_text() {
        assert_eq!(decode_html_entities("hello world"), "hello world");
    }

    #[test]
    fn strip_xssi_removes_prefixes() {
        assert_eq!(strip_xssi(")]}'\nbody"), "body");
        assert_eq!(strip_xssi(")]}'body"), "body");
        assert_eq!(strip_xssi("body"), "body");
    }

    #[test]
    fn json_path_traverses_nested_arrays() {
        let value = serde_json::json!([null, [null, null, "target"]]);
        assert_eq!(
            json_path(&value, &[1, 2]).and_then(|v| v.as_str()),
            Some("target")
        );
        assert_eq!(json_path(&value, &[1, 5]), None);
    }

    #[test]
    fn json_path_string_returns_none_for_empty() {
        let value = serde_json::json!([null, ""]);
        assert_eq!(json_path_string(&value, &[1]), None);
    }

    #[test]
    fn extract_target_url_passes_absolute_urls() {
        assert_eq!(
            extract_target_url("https://example.com/page"),
            Some("https://example.com/page".to_string())
        );
    }

    #[test]
    fn extract_target_url_parses_google_redirect() {
        let result = extract_target_url("/url?q=https://example.com&sa=U");
        assert_eq!(result, Some("https://example.com".to_string()));
    }

    #[test]
    fn extract_places_api_place_id_finds_in_query() {
        let id = extract_places_api_place_id(
            "https://www.google.com/maps/place/?q=place_id:ChIJSANITIZED",
        );
        assert_eq!(id.as_deref(), Some("ChIJSANITIZED"));
    }

    #[test]
    fn extract_places_api_place_id_finds_in_places_prefix() {
        let id =
            extract_places_api_place_id("https://places.googleapis.com/v1/places/ChIJSANITIZED");
        assert_eq!(id.as_deref(), Some("ChIJSANITIZED"));
    }

    #[test]
    fn extract_places_api_place_id_returns_none_for_knowledge_graph() {
        let id = extract_places_api_place_id(
            "https://www.google.com/maps/place/Test?place_id=%2Fg%2F11jz959qng",
        );
        assert_eq!(id, None);
    }

    #[test]
    fn normalize_places_api_place_id_strips_prefixes() {
        assert_eq!(
            normalize_places_api_place_id("places/ChIJabc"),
            Some("ChIJabc")
        );
        assert_eq!(normalize_places_api_place_id(" ChIJabc "), Some("ChIJabc"));
        assert_eq!(normalize_places_api_place_id("/g/abc"), None);
        assert_eq!(normalize_places_api_place_id(""), None);
    }

    #[test]
    fn extract_place_id_from_query_handles_variants() {
        let id =
            extract_place_id_from_query("https://www.google.com/maps/place/?q=place_id:ChIJabc");
        assert_eq!(id.as_deref(), Some("ChIJabc"));

        let id = extract_place_id_from_query("https://www.google.com/maps/place/?place_id=ChIJabc");
        assert_eq!(id.as_deref(), Some("ChIJabc"));
    }

    #[test]
    fn extract_place_id_after_finds_marker() {
        assert_eq!(
            extract_place_id_after("!1s0x1234:0x5678", "!1s"),
            Some("0x1234:0x5678".to_string())
        );
        assert_eq!(extract_place_id_after("no marker", "!1s"), None);
    }

    #[test]
    fn extract_coordinates_from_query_with_q_param() {
        let coords =
            extract_coordinates_from_query("https://www.google.com/maps/place?q=25.1972,55.2744");
        assert_eq!(coords, Some((25.1972, 55.2744)));
    }

    #[test]
    fn extract_coordinates_from_data_params_test() {
        let coords = extract_coordinates_from_data_params("data=!3d24.169194!4d120.646361");
        assert_eq!(coords, Some((24.169194, 120.646361)));
    }

    #[test]
    fn extract_number_after_works_with_marker() {
        let num = extract_number_after("!3d24.169194", "!3d");
        assert!((num.unwrap() - 24.169194).abs() < 1e-6);
        assert_eq!(extract_number_after("no marker", "!3d"), None);
    }

    #[test]
    fn parse_coordinates_handles_decimal() {
        let coords = parse_coordinates("24.169194, 120.646361");
        assert_eq!(coords, Some((24.169194, 120.646361)));
    }

    #[test]
    fn parse_coordinates_handles_dms() {
        let coords = parse_coordinates("24°10'09.1\"N 120°38'46.9\"E");
        assert!(coords.is_some());
        let (lat, lng) = coords.unwrap();
        assert!((lat - 24.169194).abs() < 0.00001);
        assert!((lng - 120.646361).abs() < 0.00001);
    }

    #[test]
    fn parse_dms_coordinates_rejects_invalid() {
        assert_eq!(parse_dms_coordinates("invalid"), None);
    }

    #[test]
    fn valid_lat_lng_checks_bounds() {
        assert!(valid_lat_lng(0.0, 0.0));
        assert!(valid_lat_lng(90.0, 180.0));
        assert!(valid_lat_lng(-90.0, -180.0));
        assert!(!valid_lat_lng(91.0, 0.0));
        assert!(!valid_lat_lng(0.0, 181.0));
        assert!(!valid_lat_lng(f64::NAN, 0.0));
    }

    #[test]
    fn extract_google_maps_uris_finds_google_urls() {
        let urls = extract_google_maps_uris(
            "visit https://www.google.com/maps/place/Test and https://maps.app.goo.gl/abc",
        );
        assert_eq!(urls.len(), 2);
        assert!(urls[0].contains("google.com/maps"));
        assert!(urls[1].contains("maps.app.goo.gl"));
    }

    #[test]
    fn extract_google_maps_uris_returns_empty_for_no_match() {
        let urls = extract_google_maps_uris("no urls here");
        assert!(urls.is_empty());
    }

    #[test]
    fn format_rating_in_places_api_rounds_correctly() {
        assert_eq!(format_rating(4.0), "4");
        assert_eq!(format_rating(4.5), "4.5");
        assert_eq!(format_rating(4.55), "4.6");
    }

    #[test]
    fn has_any_data_returns_false_for_empty() {
        let details = PlaceApiDetails {
            id: None,
            display_name: None,
            latitude: None,
            longitude: None,
            rating: None,
            website: None,
            google_maps_url: None,
            description: None,
            google_place_details: None,
        };
        assert!(!details.has_any_data());
    }

    #[test]
    fn parses_places_api_text_search_response() {
        let response: TextSearchResponse = serde_json::from_value(serde_json::json!({
            "places": [{
                "id": "places/ChIJSANITIZED",
                "displayName": {"text": "測試地點"},
                "location": {"latitude": 24.1, "longitude": 120.2},
                "rating": 4.7,
                "websiteUri": "https://example.com",
                "googleMapsUri": "https://maps.google.com/?cid=123",
                "businessStatus": "OPERATIONAL",
                "primaryType": "restaurant",
                "primaryTypeDisplayName": {"text": "餐廳", "languageCode": "zh-TW"},
                "editorialSummary": {"text": "測試簡介"}
            }]
        }))
        .unwrap();

        let details = response
            .places
            .into_iter()
            .find_map(PlaceApiDetails::from_place_value)
            .unwrap();

        assert_eq!(details.display_name.as_deref(), Some("測試地點"));
        assert_eq!(details.latitude, Some(24.1));
        assert_eq!(details.longitude, Some(120.2));
        assert_eq!(
            details.google_maps_url.as_deref(),
            Some("https://maps.google.com/?cid=123")
        );
        assert_eq!(details.rating, None);
        assert_eq!(details.website, None);
        assert_eq!(details.description, None);
        let google_place_details = details.google_place_details.as_ref().unwrap();
        assert_eq!(
            google_place_details
                .get("primaryType")
                .and_then(JsonValue::as_str),
            Some("restaurant")
        );
        assert_eq!(
            google_place_details
                .get("businessStatus")
                .and_then(JsonValue::as_str),
            Some("OPERATIONAL")
        );
    }

    #[test]
    fn parses_google_maps_preview_metadata_response() {
        let mut root = vec![JsonValue::Null; 7];
        root[4] = serde_json::json!([[3683.1, 121.0055328, 22.6122787]]);

        let mut place = vec![JsonValue::Null; 176];
        let mut rating = vec![JsonValue::Null; 8];
        rating[7] = serde_json::json!(4.6);
        place[4] = JsonValue::Array(rating);
        place[7] = serde_json::json!([
            "/url?q=https://www.facebook.com/profile.php%3Fid%3D100069902162846&sa=U"
        ]);
        place[11] = serde_json::json!("太麻里福興宮福德正神(招財貓土地公廟)");
        place[32] = serde_json::json!([null, [null, "太麻里福興宮是座歷史悠久的土地公廟"]]);
        place[42] = serde_json::json!("https://www.google.com/maps/preview/place/test");
        root[6] = JsonValue::Array(place);

        let details = PlaceApiDetails::from_google_maps_preview(&JsonValue::Array(root)).unwrap();

        assert_eq!(
            details.display_name.as_deref(),
            Some("太麻里福興宮福德正神(招財貓土地公廟)")
        );
        assert_eq!(details.rating.as_deref(), Some("4.6"));
        assert_eq!(
            details.website.as_deref(),
            Some("https://www.facebook.com/profile.php?id=100069902162846")
        );
        assert_eq!(details.latitude, Some(22.6122787));
        assert_eq!(details.longitude, Some(121.0055328));
        assert_eq!(
            details.description.as_deref(),
            Some("太麻里福興宮是座歷史悠久的土地公廟")
        );
    }

    #[test]
    fn applies_only_missing_places_api_fields() {
        let mut place = GooglePlace {
            title: Some("Original".to_string()),
            notes: None,
            url: None,
            tags: None,
            comments: None,
            latitude: Some("1.0".to_string()),
            longitude: None,
            place_name: None,
            rating: None,
            website: None,
            description: None,
            original_name: None,
            place_id: None,
            google_place_details: None,
        };

        apply_places_api_details(
            &mut place,
            PlaceApiDetails {
                id: Some("places/ChIJSANITIZED".to_string()),
                display_name: Some("API Name".to_string()),
                latitude: Some(2.0),
                longitude: Some(3.0),
                rating: Some("4.5".to_string()),
                website: Some("https://example.com".to_string()),
                google_maps_url: Some("https://maps.google.com/?cid=123".to_string()),
                description: Some("API description".to_string()),
                google_place_details: Some(serde_json::json!({
                    "id": "places/ChIJSANITIZED",
                    "displayName": {"text": "API Name"}
                })),
            },
        );

        assert_eq!(place.latitude.as_deref(), Some("1.0"));
        assert_eq!(place.longitude.as_deref(), Some("3"));
        assert_eq!(place.original_name.as_deref(), Some("Original"));
        assert_eq!(place.place_name.as_deref(), Some("API Name"));
        assert_eq!(place.rating.as_deref(), Some("4.5"));
        assert_eq!(place.website.as_deref(), Some("https://example.com"));
        assert_eq!(
            place.url.as_deref(),
            Some("https://maps.google.com/?cid=123")
        );
        assert!(place.google_place_details.is_some());
    }

    #[test]
    fn skips_places_that_already_have_basic_umap_fields() {
        let place = GooglePlace {
            title: Some("Saved title".to_string()),
            notes: None,
            url: Some("https://www.google.com/maps/place/?q=place_id:ChIJSANITIZED".to_string()),
            tags: None,
            comments: None,
            latitude: Some("24.1".to_string()),
            longitude: Some("120.2".to_string()),
            place_name: Some("Existing place name".to_string()),
            rating: None,
            website: None,
            description: None,
            original_name: None,
            place_id: Some("ChIJSANITIZED".to_string()),
            google_place_details: None,
        };

        assert!(!needs_places_api_enrichment(&place));
    }

    #[test]
    fn enriches_places_missing_basic_umap_fields() {
        let place = GooglePlace {
            title: Some("Saved title".to_string()),
            notes: None,
            url: Some("https://www.google.com/maps/place/?q=place_id:ChIJSANITIZED".to_string()),
            tags: None,
            comments: None,
            latitude: Some("24.1".to_string()),
            longitude: None,
            place_name: Some("Existing place name".to_string()),
            rating: None,
            website: None,
            description: None,
            original_name: None,
            place_id: Some("ChIJSANITIZED".to_string()),
            google_place_details: None,
        };

        assert!(needs_places_api_enrichment(&place));
    }

    #[test]
    fn merges_google_pro_properties_when_text_search_location_matches() {
        let existing = PlaceApiDetails {
            id: None,
            display_name: Some("Existing".to_string()),
            latitude: Some(24.7994433),
            longitude: Some(120.9730098),
            rating: None,
            website: None,
            google_maps_url: None,
            description: None,
            google_place_details: None,
        };
        let details = PlaceApiDetails {
            id: Some("places/ChIJSANITIZED".to_string()),
            display_name: Some("API".to_string()),
            latitude: Some(24.7994434),
            longitude: Some(120.9730099),
            rating: None,
            website: None,
            google_maps_url: Some("https://maps.google.com/?cid=123".to_string()),
            description: None,
            google_place_details: Some(serde_json::json!({"id": "places/ChIJSANITIZED"})),
        };

        assert!(existing.can_merge_google_place_details(&details));
    }

    #[test]
    fn rejects_google_pro_properties_when_text_search_location_differs() {
        let existing = PlaceApiDetails {
            id: None,
            display_name: Some("Existing".to_string()),
            latitude: Some(24.7994433),
            longitude: Some(120.9730098),
            rating: None,
            website: None,
            google_maps_url: None,
            description: None,
            google_place_details: None,
        };
        let details = PlaceApiDetails {
            id: Some("places/ChIJSANITIZED".to_string()),
            display_name: Some("Wrong".to_string()),
            latitude: Some(24.7850264),
            longitude: Some(121.0132933),
            rating: None,
            website: None,
            google_maps_url: Some("https://maps.google.com/?cid=123".to_string()),
            description: None,
            google_place_details: Some(serde_json::json!({"id": "places/ChIJSANITIZED"})),
        };

        assert!(!existing.can_merge_google_place_details(&details));
    }

    #[test]
    fn normalizes_supported_locale_values() {
        assert_eq!(normalize_locale(""), "en");
        assert_eq!(normalize_locale("en-US"), "en");
        assert_eq!(normalize_locale("zh-hant-tw"), "zh-TW");
        assert_eq!(normalize_locale("ja-JP"), "ja-JP");
    }

    #[test]
    fn builds_accept_language_from_locale() {
        assert_eq!(accept_language("zh-TW"), "zh-TW,zh;q=0.9,en;q=0.8");
        assert_eq!(accept_language("en-US"), "en,zh-TW;q=0.6");
        assert_eq!(accept_language("ja-JP"), "ja-JP,en;q=0.8");
    }

    #[test]
    fn preserves_existing_original_name() {
        let mut place = GooglePlace {
            title: Some("Source title".to_string()),
            notes: None,
            url: None,
            tags: None,
            comments: None,
            latitude: None,
            longitude: None,
            place_name: None,
            rating: None,
            website: None,
            description: None,
            original_name: Some("Existing original".to_string()),
            place_id: None,
            google_place_details: None,
        };

        apply_places_api_details(
            &mut place,
            PlaceApiDetails {
                id: None,
                display_name: Some("Localized name".to_string()),
                latitude: None,
                longitude: None,
                rating: None,
                website: None,
                google_maps_url: None,
                description: None,
                google_place_details: None,
            },
        );

        assert_eq!(place.original_name.as_deref(), Some("Existing original"));
        assert_eq!(place.place_name.as_deref(), Some("Localized name"));
    }

    #[test]
    fn parses_dms_coordinate_query() {
        let (lat, lng) = parse_coordinates("24°10'09.1\"N 120°38'46.9\"E").unwrap();

        assert!((lat - 24.169194).abs() < 0.00001);
        assert!((lng - 120.646361).abs() < 0.00001);
    }

    #[test]
    fn parses_decimal_coordinate_query() {
        let (lat, lng) = parse_coordinates("24.169194, 120.646361").unwrap();

        assert_eq!(lat, 24.169194);
        assert_eq!(lng, 120.646361);
    }

    #[test]
    fn extracts_places_api_place_id_from_google_maps_query_uri() {
        let place_id = extract_places_api_place_id(
            "https://www.google.com/maps/place/?q=place_id:ChIJSANITIZED",
        );

        assert_eq!(place_id.as_deref(), Some("ChIJSANITIZED"));
    }

    #[test]
    fn ignores_legacy_google_maps_feature_id_as_places_api_place_id() {
        let place_id = extract_places_api_place_id(
            "https://www.google.com/maps/place/Test/data=!4m2!3m1!1s0x346917cd034a5009:0xc2acf59feee6e3a5",
        );

        assert_eq!(place_id, None);
    }

    #[test]
    fn ignores_google_knowledge_graph_ids_as_places_api_place_ids() {
        assert_eq!(normalize_places_api_place_id("/g/11jz959qng"), None);
        assert_eq!(normalize_places_api_place_id("/m/03h3wxp"), None);

        assert_eq!(
            extract_places_api_place_id(
                "https://www.google.com/maps/place/Test?place_id=%2Fg%2F11jz959qng"
            ),
            None
        );
        assert_eq!(
            extract_places_api_place_id(
                "https://www.google.com/maps/place/Test?query_place_id=%2Fm%2F03h3wxp"
            ),
            None
        );
    }

    #[test]
    fn extracts_coordinates_from_legacy_google_maps_feature_id() {
        let (lat, lng) = extract_feature_id_coordinates(
            "https://www.google.com/maps/place/%E5%A4%AA%E9%BA%BB%E9%87%8C%E7%A6%8F%E8%88%88%E5%AE%AE%E7%A6%8F%E5%BE%B7%E6%AD%A3%E7%A5%9E(%E6%8B%9B%E8%B2%A1%E8%B2%93%E5%9C%9F%E5%9C%B0%E5%85%AC%E5%BB%9F)/data=!4m2!3m1!1s0x346fce93f16adf79:0x736c7ae8d9b6d5a",
        )
        .unwrap();

        assert!((lat - 22.611850681229942).abs() < 1e-12);
        assert!((lng - 121.00582893510104).abs() < 1e-12);
    }

    #[test]
    fn extracts_coordinates_from_google_maps_at_uri() {
        let (lat, lng) = extract_coordinates_from_google_maps_uri(
            "https://www.google.com/maps/place/Test/@24.169194,120.646361,17z",
        )
        .unwrap();

        assert_eq!(lat, 24.169194);
        assert_eq!(lng, 120.646361);
    }

    #[test]
    fn extracts_coordinates_from_google_maps_data_params() {
        let (lat, lng) = extract_coordinates_from_google_maps_uri(
            "https://www.google.com/maps/place/Test/data=!3d24.169194!4d120.646361",
        )
        .unwrap();

        assert_eq!(lat, 24.169194);
        assert_eq!(lng, 120.646361);
    }

    #[test]
    fn extracts_coordinates_from_google_maps_staticmap_html() {
        let html = r#"
            <meta content="https://maps.google.com/maps/api/staticmap?center=24.1638431%2C120.7238656&amp;zoom=15" property="og:image">
        "#;
        let (lat, lng) = extract_coordinates_from_google_maps_html(html).unwrap();

        assert_eq!(lat, 24.1638431);
        assert_eq!(lng, 120.7238656);
    }

    #[test]
    fn extracts_coordinates_from_google_maps_preview_pb_html() {
        let html = r#"
            href="/maps/preview/place?pb=!1m3!1d14560.89929404038!2d120.7238656!3d24.163843099999998!4f13.1"
        "#;
        let (lat, lng) = extract_coordinates_from_google_maps_html(html).unwrap();

        assert_eq!(lat, 24.163843099999998);
        assert_eq!(lng, 120.7238656);
    }

    #[tokio::test]
    async fn resolves_coordinate_query_without_text_search() {
        let client = PlacesApiClient::new("unused-key");
        let place = GooglePlace {
            title: Some("24°10'09.1\"N 120°38'46.9\"E".to_string()),
            notes: None,
            url: None,
            tags: None,
            comments: None,
            latitude: None,
            longitude: None,
            place_name: None,
            rating: None,
            website: None,
            description: None,
            original_name: None,
            place_id: None,
            google_place_details: None,
        };

        let details = client.resolve_place(&place).await.unwrap().unwrap();

        assert!((details.latitude.unwrap() - 24.169194).abs() < 0.00001);
        assert!((details.longitude.unwrap() - 120.646361).abs() < 0.00001);
        assert!(
            details
                .google_maps_url
                .as_deref()
                .is_some_and(|url| url.starts_with("https://www.google.com/maps/search/24.169194"))
        );
    }
}
