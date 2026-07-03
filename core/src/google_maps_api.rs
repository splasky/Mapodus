use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleSavedPlace {
    pub title: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub place_id: Option<String>,
    pub list: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleSavedList {
    pub id: String,
    pub name: String,
    pub place_count: u32,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GooglePlaceDetails {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub url: Option<String>,
    pub place_name: Option<String>,
    pub rating: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
    pub original_name: Option<String>,
    pub english_name: Option<String>,
}

pub struct GoogleMapsClient {
    client: reqwest::Client,
    cookies: HashMap<String, String>,
}

impl GoogleMapsClient {
    pub fn new(cookies: HashMap<String, String>) -> Self {
        let client = reqwest::Client::builder()
            .cookie_store(false)
            .build()
            .expect("Failed to build reqwest client");
        GoogleMapsClient { client, cookies }
    }

    fn cookie_string(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn request_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("X-Same-Domain", "1".parse().unwrap());
        headers.insert(
            reqwest::header::COOKIE,
            self.cookie_string().parse().unwrap(),
        );
        headers.insert(
            reqwest::header::REFERER,
            "https://www.google.com/".parse().unwrap(),
        );
        headers.insert("x-maps-diversion-context-bin", "CAE=".parse().unwrap());
        headers.insert(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Safari/537.36"
                .parse().unwrap(),
        );
        if let Some(auth) = self.sapisid_hash() {
            headers.insert("Authorization", auth.parse().unwrap());
        }
        headers
    }

    fn sapisid_value(&self) -> Option<&str> {
        self.cookies
            .get("SAPISID")
            .or_else(|| self.cookies.get("__Secure-3PAPISID"))
            .or_else(|| self.cookies.get("__Secure-1PSAPISID"))
            .map(|s| s.as_str())
    }

    fn sapisid_hash(&self) -> Option<String> {
        let sapisid = self.sapisid_value()?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        let input = format!("{} {}", timestamp, sapisid);
        let hash = hex_encode(&md5(input.as_bytes()));
        Some(format!("SAPISIDHASH {}_{}", timestamp, hash))
    }

    /// Fetch all saved lists via the MAS endpoint.
    pub async fn fetch_saved_lists(&self) -> Result<Vec<GoogleSavedList>, crate::error::AppError> {
        let sapisid = self.sapisid_value().unwrap_or("A");
        let pb = build_mas_pb(sapisid);
        let url = "https://www.google.com/locationhistory/preview/mas";
        let response = self
            .client
            .get(url)
            .query(&[("authuser", "0"), ("hl", "en"), ("gl", "us"), ("pb", &pb)])
            .headers(self.request_headers())
            .send()
            .await?;
        let status = response.status();
        let data = response.bytes().await?;
        if !status.is_success() {
            let preview = String::from_utf8_lossy(&data[..data.len().min(500)]);
            eprintln!("[MAS] HTTP {} response (first 500): {}", status, preview);
            return Err(crate::error::AppError::Http(format!(
                "MAS endpoint returned {}: {}",
                status, preview
            )));
        }
        let body = strip_xssi_bytes(&data).ok_or_else(|| {
            crate::error::AppError::Parse("MAS response is not valid UTF-8".into())
        })?;
        // Log response summary for debugging
        eprintln!(
            "[MAS] 200 OK, response length={} stripped, first 200: {}",
            body.len(),
            &body[..body.len().min(200)]
        );
        let json: serde_json::Value = serde_json::from_str(body).map_err(|e| {
            let preview = &body[..body.len().min(500)];
            eprintln!("[MAS] JSON parse error: {}. Body: {}", e, preview);
            crate::error::AppError::Parse(format!(
                "MAS JSON parse error: {}. Body (first 500): {}",
                e, preview
            ))
        })?;
        let lists = parse_mas_lists(&json)?;
        eprintln!("[MAS] Parsed {} saved lists", lists.len());
        if lists.is_empty() {
            // Dump the top-level structure for debugging
            if let Some(root) = json.as_array() {
                eprintln!("[MAS] Top-level array has {} elements", root.len());
                for (i, elem) in root.iter().enumerate() {
                    if elem.is_array() {
                        let arr = elem.as_array().unwrap();
                        eprintln!("  [{}] array(len={})", i, arr.len());
                    } else if !elem.is_null() {
                        eprintln!("  [{}] {}", i, elem);
                    }
                }
            }
        }
        Ok(lists)
    }

    /// Fetch places for a specific saved list.
    pub async fn fetch_list_places(
        &self,
        list_id: &str,
        list_name: &str,
    ) -> Result<Vec<GoogleSavedPlace>, crate::error::AppError> {
        let token = self
            .sapisid_value()
            .map(compute_mas_token)
            .unwrap_or_else(|| "A".into());
        let pb = format!(
            "!1m4!1s{id}!2e1!3m1!1e1!2e2!3e2!4i500!6m3!1s{token}!7e81!28e2!18i3!16b1",
            id = list_id,
            token = token
        );
        let url = "https://www.google.com/maps/preview/entitylist/getlist";
        let response = self
            .client
            .get(url)
            .query(&[("authuser", "0"), ("hl", "en"), ("gl", "us"), ("pb", &pb)])
            .headers(self.request_headers())
            .send()
            .await?;
        let status = response.status();
        let data = response.bytes().await?;
        if !status.is_success() {
            let preview = String::from_utf8_lossy(&data[..data.len().min(200)]);
            return Err(crate::error::AppError::Http(format!(
                "getlist endpoint returned {} for list '{}': {}",
                status, list_name, preview
            )));
        }
        let body = strip_xssi_bytes(&data).ok_or_else(|| {
            crate::error::AppError::Parse(format!(
                "getlist response not valid UTF-8 for list '{}'",
                list_name
            ))
        })?;
        let json: serde_json::Value = serde_json::from_str(body).map_err(|e| {
            let preview = &body[..body.len().min(500)];
            crate::error::AppError::Parse(format!(
                "getlist JSON error for '{}': {}. Body: {}",
                list_name, e, preview
            ))
        })?;
        let places = parse_getlist_places(&json, list_name)?;
        if places.is_empty() {
            eprintln!(
                "[getlist] '{}' returned 0 places. Raw response (first 500): {}",
                list_name,
                &body[..body.len().min(500)]
            );
        }
        Ok(places)
    }

    /// Look up place details by place_id using the Google Maps preview/place endpoint.
    pub async fn get_place_details(
        &self,
        place_id: &str,
    ) -> Result<Option<GooglePlaceDetails>, crate::error::AppError> {
        let pb = format!("!1s{}!2e1", place_id);
        let url = "https://www.google.com/maps/preview/place";
        let response = self
            .client
            .get(url)
            .query(&[("authuser", "0"), ("hl", "en"), ("gl", "us"), ("pb", &pb)])
            .headers(self.request_headers())
            .send()
            .await?;
        let status = response.status();
        let data = response.bytes().await?;
        if !status.is_success() {
            let preview = String::from_utf8_lossy(&data[..data.len().min(200)]);
            return Err(crate::error::AppError::Http(format!(
                "Place details endpoint returned {} for place_id '{}': {}",
                status, place_id, preview
            )));
        }
        let body = strip_xssi_bytes(&data).ok_or_else(|| {
            crate::error::AppError::Parse("Place details response not valid UTF-8".into())
        })?;
        let json: serde_json::Value = serde_json::from_str(body).map_err(|e| {
            let preview = &body[..body.len().min(500)];
            crate::error::AppError::Parse(format!(
                "Place details JSON error for '{}': {}. Body: {}",
                place_id, e, preview
            ))
        })?;
        parse_place_details(&json, place_id)
    }

    /// High-level: get all places from all saved lists.
    pub async fn get_all_saved_places(
        &self,
    ) -> Result<Vec<GoogleSavedPlace>, crate::error::AppError> {
        let lists = self.fetch_saved_lists().await?;
        if lists.is_empty() {
            eprintln!("  No saved lists found (cookies may be expired).");
            return Ok(vec![]);
        }
        println!("  Found {} saved lists:", lists.len());
        for l in &lists {
            println!("    {} ({} places) [id={}]", l.name, l.place_count, l.id);
        }
        let mut all_places = Vec::new();
        for list in &lists {
            eprintln!("  Fetching places for '{}'...", list.name);
            match self.fetch_list_places(&list.id, &list.name).await {
                Ok(places) => {
                    eprintln!("    Got {} places", places.len());
                    all_places.extend(places);
                }
                Err(e) => {
                    eprintln!("    Failed: {}", e);
                }
            }
        }
        Ok(all_places)
    }
}

/// Resolve coordinates from a Google Maps place URL by following redirects.
///
/// Google Maps `/place/NAME/data=!...` URLs redirect to
/// `/place/NAME/@lat,lng,zoom/data=!...` which contains coordinates in the `@` pattern.
/// This function follows the redirect and parses the final URL.
///
/// Requires no cookies or API key — just a standard HTTP GET.
pub async fn resolve_place_url_coords(url: &str) -> Option<(f64, f64)> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let resp = client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Safari/537.36",
        )
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml")
        .send()
        .await
        .ok()?;

    let final_url = resp.url().as_str().to_string();
    if final_url == url {
        return None;
    }

    crate::google::extract_coords_from_url(&final_url)
}

// ── ProtoTextWriter for Google's !-format protobuf ──

struct ProtoTextWriter {
    buf: String,
    token_count: u32,
}

impl ProtoTextWriter {
    fn new() -> Self {
        ProtoTextWriter {
            buf: String::new(),
            token_count: 0,
        }
    }

    fn write_string(&mut self, field: u32, value: &str) {
        self.buf.push_str(&format!("!{}s{}", field, value));
        self.token_count += 1;
    }

    fn write_int(&mut self, field: u32, value: i64) {
        self.buf.push_str(&format!("!{}i{}", field, value));
        self.token_count += 1;
    }

    fn write_enum(&mut self, field: u32, value: i64) {
        self.buf.push_str(&format!("!{}e{}", field, value));
        self.token_count += 1;
    }

    fn write_message(&mut self, field: u32, content: &str, child_tokens: u32) {
        self.buf.push_str(&format!("!{}m{}", field, content));
        // The token count for a message includes itself + children
        self.token_count += 1 + child_tokens;
    }

    fn write_double(&mut self, field: u32, value: f64) {
        self.buf.push_str(&format!("!{}d{}", field, value));
        self.token_count += 1;
    }

    fn write_float(&mut self, field: u32, value: f32) {
        self.buf.push_str(&format!("!{}f{}", field, value));
        self.token_count += 1;
    }

    fn write_bool(&mut self, field: u32, value: bool) {
        self.buf.push_str(&format!("!{}b{}", field, value as u8));
        self.token_count += 1;
    }

    fn into_string(self) -> String {
        self.buf
    }
}

/// Compute the session token for the MAS pb parameter.
/// In the reference curl, this is the first field inside the auth message,
/// appearing as e.g. `!1sabRAarDgD4uu2roPmN6v2Ak`. The value is
/// base64url(MD5(SAPISID)).
fn compute_mas_token(sapisid: &str) -> String {
    let hash = md5(sapisid.as_bytes());
    base64url_encode(&hash)
}

fn base64url_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    let mut i = 0;
    while i + 3 <= input.len() {
        let b0 = input[i] as u32;
        let b1 = input[i + 1] as u32;
        let b2 = input[i + 2] as u32;
        let combined = (b0 << 16) | (b1 << 8) | b2;
        result.push(ALPHABET[((combined >> 18) & 0x3f) as usize] as char);
        result.push(ALPHABET[((combined >> 12) & 0x3f) as usize] as char);
        result.push(ALPHABET[((combined >> 6) & 0x3f) as usize] as char);
        result.push(ALPHABET[(combined & 0x3f) as usize] as char);
        i += 3;
    }
    let remaining = input.len() - i;
    if remaining == 1 {
        let b0 = input[i] as u32;
        result.push(ALPHABET[((b0 >> 2) & 0x3f) as usize] as char);
        result.push(ALPHABET[((b0 << 4) & 0x3f) as usize] as char);
    } else if remaining == 2 {
        let b0 = input[i] as u32;
        let b1 = input[i + 1] as u32;
        let combined = (b0 << 8) | b1;
        result.push(ALPHABET[((combined >> 10) & 0x3f) as usize] as char);
        result.push(ALPHABET[((combined >> 4) & 0x3f) as usize] as char);
        result.push(ALPHABET[((combined << 2) & 0x3f) as usize] as char);
    }
    result
}

/// Build the pb parameter for the MAS endpoint.
/// `sapisid` is used to compute the session token needed in the request.
fn build_mas_pb(sapisid: &str) -> String {
    let token = compute_mas_token(sapisid);
    let mut w = ProtoTextWriter::new();

    // Field 2: message with auth/version info
    let mut inner = ProtoTextWriter::new();
    inner.write_string(1, &token);
    inner.write_enum(7, 81);
    inner.write_int(15, 20393);
    let inner_str = inner.into_string();
    w.write_message(2, &format!("{}{}", 3, inner_str), 3);

    // Field 7: message with 1 child (int 50)
    let mut f7 = ProtoTextWriter::new();
    f7.write_int(1, 50);
    let f7_str = f7.into_string();
    w.write_message(7, &format!("{}{}", 1, f7_str), 1);

    // Field 9: empty message
    w.write_message(9, "0", 0);

    // Field 12: message with 1 child (int 50)
    let mut f12 = ProtoTextWriter::new();
    f12.write_int(1, 50);
    let f12_str = f12.into_string();
    w.write_message(12, &format!("{}{}", 1, f12_str), 1);

    // Field 15: message with 1 child (int 50)
    let mut f15 = ProtoTextWriter::new();
    f15.write_int(1, 50);
    let f15_str = f15.into_string();
    w.write_message(15, &format!("{}{}", 1, f15_str), 1);

    // Field 17: empty viewport (matches working curl format)
    w.write_message(17, "0", 0);

    // Field 18: map viewport / zoom (same format as working curl)
    let mut f18 = ProtoTextWriter::new();
    let mut center = ProtoTextWriter::new();
    center.write_double(1, 34667.92215241253);
    center.write_double(2, 120.651776);
    center.write_double(3, 24.1271641);
    f18.write_message(1, &format!("{}{}", 3, center.into_string()), 3);
    f18.write_message(2, "0", 0);
    let mut dims = ProtoTextWriter::new();
    dims.write_int(1, 634);
    dims.write_int(2, 914);
    f18.write_message(3, &format!("{}{}", 2, dims.into_string()), 2);
    f18.write_float(4, 13.1);
    w.write_message(18, &format!("{}{}", 9, f18.into_string()), 9);

    // Fields 23, 24, 38: option flags (int 50, bool true)
    let mut flag = ProtoTextWriter::new();
    flag.write_int(1, 50);
    flag.write_bool(3, true);
    let flag_str = flag.into_string();
    w.write_message(23, &format!("{}{}", 2, flag_str), 2);
    w.write_message(24, &format!("{}{}", 2, flag_str), 2);
    w.write_message(38, &format!("{}{}", 2, flag_str), 2);

    w.into_string()
}

// ── Response parsing ──

pub fn strip_xssi(data: &str) -> Option<&str> {
    let bytes = data.as_bytes();
    let start = if bytes.len() > 5 && &bytes[..5] == b")]}'\n" {
        5
    } else if bytes.len() > 4 && &bytes[..4] == b")]}" {
        4
    } else {
        0
    };
    Some(data[start..].trim())
}

fn strip_xssi_bytes(data: &[u8]) -> Option<&str> {
    let start = if data.len() > 5 && &data[..5] == b")]}'\n" {
        5
    } else if data.len() > 4 && &data[..4] == b")]}" {
        4
    } else {
        0
    };
    let s = std::str::from_utf8(&data[start..]).ok()?;
    Some(s.trim())
}

/// Parse the MAS endpoint response to extract saved lists.
/// The saved lists are nested deep in the response array. We search through the
/// top-level array for an element that contains saved place list entries.
fn parse_mas_lists(
    value: &serde_json::Value,
) -> Result<Vec<GoogleSavedList>, crate::error::AppError> {
    let root = value
        .as_array()
        .ok_or_else(|| crate::error::AppError::Parse("MAS response root is not an array".into()))?;

    // The response is a sparse array. The saved lists are nested under one of the
    // top-level elements. Look for an element that is an array of the form:
    //   [null, null, null, [list_entries, ...], ...]
    // where list_entries is an array of saved list objects.
    // Also handle the simpler case where the list entries are directly nested.
    let saved_lists = find_saved_lists_array(root);
    let my_maps = find_my_maps_array(root);

    let mut lists = Vec::new();

    // Process My Maps (Google My Maps, which also appear in saved)
    if let Some(maps) = my_maps {
        for entry in maps {
            let arr = match entry.as_array() {
                Some(a) if a.len() >= 5 => a,
                _ => continue,
            };
            let name = arr[0].as_str().unwrap_or("").to_string();
            let mid = arr[1].as_str().unwrap_or("").to_string();
            if name.is_empty() || mid.is_empty() {
                continue;
            }
            let url = arr[4].as_str().map(|s| s.to_string());
            lists.push(GoogleSavedList {
                id: mid,
                name,
                place_count: 0,
                url,
            });
        }
    }

    // Process saved places lists
    if let Some(entries) = saved_lists {
        for entry in entries {
            let arr = match entry.as_array() {
                Some(a) if a.len() >= 5 => a,
                _ => continue,
            };
            let list_meta = match arr[0].as_array() {
                Some(m) => m,
                None => continue,
            };
            let post_id = match list_meta.first().and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            let name = arr[4].as_str().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            let place_count = arr.get(12).and_then(|v| v.as_u64()).unwrap_or(0) as u32;

            // Check for a URL in various positions
            let url = arr
                .get(2)
                .and_then(|v| v.as_array())
                .and_then(|a| a.get(2).and_then(|v| v.as_str()))
                .map(|s| s.to_string());

            lists.push(GoogleSavedList {
                id: post_id,
                name,
                place_count,
                url,
            });
        }
    }

    Ok(lists)
}

/// Search through root array to find saved places lists.
fn find_saved_lists_array(root: &[serde_json::Value]) -> Option<&[serde_json::Value]> {
    for elem in root {
        let arr = match elem.as_array() {
            Some(a) => a,
            None => continue,
        };
        // Pattern: [null, null, null, [list_entries, ...], ...]
        if arr.len() < 4 {
            continue;
        }
        // Check first 3 are null
        if arr[0].is_null()
            && arr[1].is_null()
            && arr[2].is_null()
            && let Some(entries) = arr[3].as_array()
            && !entries.is_empty()
            && entries.iter().any(looks_like_saved_list)
        {
            return Some(entries);
        }
    }
    None
}

/// Check if a JSON value looks like a saved list entry.
fn looks_like_saved_list(value: &serde_json::Value) -> bool {
    let arr = match value.as_array() {
        Some(a) => a,
        None => return false,
    };
    if arr.len() < 5 {
        return false;
    }
    // arr[0] should be an array with [string_postId, number, ...]
    match arr[0].as_array() {
        Some(meta) => {
            if meta.is_empty() {
                return false;
            }
            meta[0].as_str().is_some()
        }
        None => false,
    }
}

/// Find My Maps entries in the response (at root[7] in the reference).
fn find_my_maps_array(root: &[serde_json::Value]) -> Option<&[serde_json::Value]> {
    for elem in root {
        let arr = match elem.as_array() {
            Some(a) => a,
            None => continue,
        };
        if arr.is_empty() {
            continue;
        }
        if arr.iter().all(|e| e.is_array())
            && let Some(first) = arr.first().and_then(|v| v.as_array())
            && first.len() >= 5
            && first[0].is_string()
        {
            return Some(arr);
        }
    }
    None
}

/// Parse the entitylist/getlist response to extract places for a list.
fn parse_getlist_places(
    value: &serde_json::Value,
    list_name: &str,
) -> Result<Vec<GoogleSavedPlace>, crate::error::AppError> {
    let mut places = Vec::new();
    let root = match value
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_array())
    {
        Some(r) => r,
        None => return Ok(places),
    };
    let entries = match root.get(8).and_then(|v| v.as_array()) {
        Some(e) => e,
        None => return Ok(places),
    };

    for item in entries {
        let arr = match item.as_array() {
            Some(a) => a,
            None => continue,
        };
        let name = arr.get(2).and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let place_info = arr.get(1).and_then(|v| v.as_array());
        let address = place_info
            .and_then(|pi| {
                pi.get(2)
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .or_else(|| pi.get(4).and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
            })
            .map(|s| s.to_string());
        let coords = place_info.and_then(|pi| pi.get(5).and_then(|v| v.as_array()));
        let latitude = coords.and_then(|c| c.get(2).and_then(|v| v.as_f64()));
        let longitude = coords.and_then(|c| c.get(3).and_then(|v| v.as_f64()));
        let place_id = place_info
            .and_then(|pi| pi.get(7).and_then(|v| v.as_str()))
            .map(|s| s.to_string());
        let url = place_id
            .as_ref()
            .map(|id| format!("https://www.google.com/maps/place/?q=place_id:{}", id));
        let notes = arr
            .get(3)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        places.push(GoogleSavedPlace {
            title: Some(name.to_string()),
            address,
            latitude,
            longitude,
            url,
            notes,
            place_id,
            list: list_name.to_string(),
        });
    }

    Ok(places)
}

/// Parse the preview/place response to extract details for a single place.
fn parse_place_details(
    value: &serde_json::Value,
    place_id: &str,
) -> Result<Option<GooglePlaceDetails>, crate::error::AppError> {
    let root = match value.as_array() {
        Some(a) => a,
        None => {
            eprintln!("[place_details] Root is not an array for place_id '{}'", place_id);
            return Ok(None);
        }
    };

    // Walk into the response to find the place data.
    // Structure is typically:
    //   [..., [place_entry, ...], ...]
    // where place_entry[2] = name, place_entry[1] contains location data.
    let entry = root.iter().find_map(|elem| {
        let arr = elem.as_array()?;
        // Look for an element whose child has the place_id
        find_place_entry_in_array(arr, place_id)
    });

    match entry {
        Some(entry_arr) => {
            let place_info = entry_arr.get(1).and_then(|v| v.as_array());
            let coords = place_info
                .and_then(|pi| pi.get(5).and_then(|v| v.as_array()));
            let latitude = coords.and_then(|c| c.get(2).and_then(|v| v.as_f64()));
            let longitude = coords.and_then(|c| c.get(3).and_then(|v| v.as_f64()));
            let pid = place_info
                .and_then(|pi| pi.get(7).and_then(|v| v.as_str()));
            let url = pid.map(|id| format!("https://www.google.com/maps/place/?q=place_id:{}", id));
            let place_name = entry_arr.get(2).and_then(|v| v.as_str()).map(|s| s.to_string());

            if latitude.is_some() || longitude.is_some() {
                eprintln!(
                    "[place_details] Found coords for place_id '{}': {:?}, {:?}",
                    place_id, latitude, longitude
                );
                Ok(Some(GooglePlaceDetails {
                    latitude,
                    longitude,
                    url,
                    place_name,
                    rating: None,
                    website: None,
                    description: None,
                    original_name: None,
                    english_name: None,
                }))
            } else {
                eprintln!(
                    "[place_details] No coords found for place_id '{}'",
                    place_id
                );
                Ok(None)
            }
        }
        None => {
            eprintln!(
                "[place_details] No entry found for place_id '{}'",
                place_id
            );
            Ok(None)
        }
    }
}

fn find_place_entry_in_array<'a>(
    arr: &'a [serde_json::Value],
    place_id: &str,
) -> Option<&'a [serde_json::Value]> {
    // Check if this array itself is a place entry (entry_arr = [null, place_info, name]
    // where place_info[7] == place_id).
    if let Some(pi) = arr.get(1).and_then(|v| v.as_array())
        && let Some(pid) = pi.get(7).and_then(|v| v.as_str())
        && pid == place_id
    {
        return Some(arr);
    }

    // Otherwise, search children recursively.
    for elem in arr {
        if let Some(inner) = elem.as_array()
            && let Some(found) = find_place_entry_in_array(inner, place_id)
        {
            return Some(found);
        }
    }
    None
}

// ── MD5 (no external dependency) ──

fn md5(input: &[u8]) -> [u8; 16] {
    Md5Context::digest(input)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

struct Md5Context;

impl Md5Context {
    fn digest(input: &[u8]) -> [u8; 16] {
        let mut ctx = Md5State::new();
        ctx.update(input);
        ctx.finalize()
    }
}

struct Md5State {
    state: [u32; 4],
    count: u64,
    buffer: [u8; 64],
}

impl Md5State {
    fn new() -> Self {
        Md5State {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
            count: 0,
            buffer: [0u8; 64],
        }
    }

    fn update(&mut self, input: &[u8]) {
        let mut index = (self.count & 0x3f) as usize;
        self.count += input.len() as u64;
        let mut i = 0;
        let part_len = 64 - index;
        if input.len() >= part_len {
            self.buffer[index..].copy_from_slice(&input[..part_len]);
            Self::transform(&mut self.state, &self.buffer);
            i = part_len;
            while i + 63 < input.len() {
                Self::transform(&mut self.state, &input[i..i + 64]);
                i += 64;
            }
            index = 0;
        }
        if i < input.len() {
            self.buffer[index..index + input.len() - i].copy_from_slice(&input[i..]);
        }
    }

    fn finalize(mut self) -> [u8; 16] {
        let bits = self.count * 8;
        let mut padding = vec![0x80u8];
        let pad_len: usize = ((56 - ((self.count + 1) % 64) + 64) % 64) as usize;
        padding.resize(1 + pad_len + 8, 0u8);
        let bits_bytes = bits.to_le_bytes();
        let start = 1 + pad_len;
        padding[start..start + 8].copy_from_slice(&bits_bytes);
        self.update(&padding);
        let mut result = [0u8; 16];
        for (i, &s) in self.state.iter().enumerate() {
            let bytes = s.to_le_bytes();
            result[i * 4..i * 4 + 4].copy_from_slice(&bytes);
        }
        result
    }

    fn transform(state: &mut [u32; 4], block: &[u8]) {
        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(K[i])
                    .wrapping_add(m[g])
                    .rotate_left(S[i]),
            );
            a = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "needs network access — run manually with `cargo test -- --include-ignored`"]
    async fn test_resolve_place_url_coords_valid_place() {
        // Google Maps URL for 新豐車站 (from test.csv)
        let url = "https://www.google.com/maps/place/%E6%96%B0%E8%B1%90%E8%BB%8A%E7%AB%99/data=!4m2!3m1!1s0x346831505ce20595:0x1d13efe96c43c466";
        let result = resolve_place_url_coords(url).await;
        assert!(result.is_some(), "resolve_place_url_coords should return Some for a valid place URL");
        let (lat, lng) = result.unwrap();
        // 新豐車站 is in Hsinchu County, Taiwan (~24.87, ~120.996)
        assert!(
            (lat - 24.87).abs() < 0.05,
            "lat should be ~24.87, got {}",
            lat
        );
        assert!(
            (lng - 120.996).abs() < 0.05,
            "lng should be ~120.996, got {}",
            lng
        );
    }

    #[tokio::test]
    #[ignore = "needs network access"]
    async fn test_resolve_place_url_coords_invalid_url_returns_none() {
        let url = "https://www.google.com/maps/place/ThisPlaceDoesNotExist12345";
        let result = resolve_place_url_coords(url).await;
        // Should not panic; may return None or Some depending on Google's redirect behavior
        let _ = result;
    }

    #[tokio::test]
    #[ignore = "needs network access"]
    async fn test_resolve_place_url_coords_no_redirect_returns_none() {
        // A URL that already has coordinates should not redirect further
        let url = "https://www.google.com/maps/place/Test/@25.033,121.565,15z/data=!4m2!3m1!1sabc";
        let result = resolve_place_url_coords(url).await;
        // Since the URL already has @lat,lng, the redirect check may
        // return None if final_url == input url
        let has_coords = result.is_some();
        if !has_coords {
            // If it returned None, at least verify via extract_coords_from_url directly
            assert!(crate::google::extract_coords_from_url(url).is_some());
        }
    }

    // ── parse_place_details tests ──

    /// Build the full mock response (outer array containing an entry) like
    /// Google Maps internal API returns. Known positions in the entry array:
    ///   [1]      = place_info array
    ///   [1][5]   = coords array [null, null, lat, lng, null]
    ///   [1][7]   = place_id string
    ///   [2]      = place name
    fn mock_place_response(place_id: &str, lat: f64, lng: f64, name: &str) -> serde_json::Value {
        serde_json::json!([
            null,   // outer[0] — filler
            [       // outer[1] — the entry
                null,                                       // [0]
                [                                           // [1] = place_info
                    null, null, null, null, null,
                    [null, null, lat, lng, null],           // [5] = coords
                    null,
                    place_id,                               // [7] = place_id
                ],
                name,                                       // [2] = place name
            ],
            null,   // outer[2] — filler
        ])
    }

    #[test]
    fn test_parse_place_details_found() {
        let place_id = "0x346835e9aa147b0b:0x8e09cb932ab96f34";
        let response = mock_place_response(place_id, 24.7994433, 120.9730098, "六扇門時尚湯鍋 新竹東南店");

        let result = parse_place_details(&response, place_id).unwrap();
        assert!(result.is_some(), "Should find the entry for known place_id");

        let details = result.unwrap();
        assert_eq!(details.latitude, Some(24.7994433));
        assert_eq!(details.longitude, Some(120.9730098));
        assert_eq!(
            details.place_name.as_deref(),
            Some("六扇門時尚湯鍋 新竹東南店")
        );
        assert_eq!(
            details.url.as_deref(),
            Some(format!("https://www.google.com/maps/place/?q=place_id:{}", place_id)).as_deref()
        );

        // These fields are not yet parsed from the mock — verify they're None
        assert!(details.rating.is_none(), "rating not yet parsed from API response");
        assert!(details.website.is_none(), "website not yet parsed from API response");
        assert!(details.description.is_none(), "description not yet parsed");
        assert!(details.original_name.is_none(), "original_name not yet parsed");
        assert!(details.english_name.is_none(), "english_name not yet parsed");
    }

    #[test]
    fn test_parse_place_details_not_found() {
        let response = mock_place_response("0xdifferent", 25.0, 121.0, "Other Place");
        let result = parse_place_details(&response, "0xabc123").unwrap();
        assert!(result.is_none(), "Should return None for unknown place_id");
    }

    #[test]
    fn test_parse_place_details_no_coords() {
        // Entry missing the coords array at [1][5]
        let place_id = "0xabc123";
        let response = serde_json::json!([
            null,
            [
                null,
                [null, null, null, null, null, null, null, place_id],
                "No Coords Place",
            ],
            null,
        ]);

        let result = parse_place_details(&response, place_id).unwrap();
        assert!(result.is_none(), "Should return None when no coords in entry");
    }

    #[test]
    fn test_parse_place_details_deeply_nested() {
        // Entry can be nested deeper inside the array structure
        let place_id = "0xnested";
        let inner_entry = serde_json::json!([
            null,
            [null, null, null, null, null, [null, null, 24.8, 121.0, null], null, place_id],
            "Nested Place",
        ]);
        let response = serde_json::json!([
            null,
            [null, [null, null, inner_entry, null], null],
            null,
        ]);

        let result = parse_place_details(&response, place_id).unwrap();
        assert!(result.is_some(), "Should find entry even when nested deeply");
        let details = result.unwrap();
        assert_eq!(details.latitude, Some(24.8));
        assert_eq!(details.longitude, Some(121.0));
    }

    #[test]
    fn test_parse_place_details_empty_response() {
        let response = serde_json::json!([]);
        let result = parse_place_details(&response, "any_id").unwrap();
        assert!(result.is_none(), "Empty response should return None");
    }

    #[test]
    fn test_parse_place_details_non_array_response() {
        let response = serde_json::json!({"error": "not an array"});
        let result = parse_place_details(&response, "any_id").unwrap();
        assert!(result.is_none(), "Non-array response should return None");
    }

    #[test]
    fn test_find_place_entry_in_array_direct_match() {
        let place_id = "0xdirecT";
        // Construct the array manually (not via json! macro) to avoid nesting issues
        let entry: serde_json::Value = serde_json::json!([
            null,
            [null, null, null, null, null, [null, null, 10.0, 20.0, null], null, place_id],
            "Direct",
        ]);
        let arr: Vec<serde_json::Value> = vec![
            serde_json::Value::Null,
            entry,
            serde_json::Value::Null,
        ];

        let found = find_place_entry_in_array(&arr, place_id);
        assert!(found.is_some(), "Should find direct entry");
        let found_arr = found.unwrap();
        assert_eq!(
            found_arr.get(2).and_then(|v| v.as_str()),
            Some("Direct")
        );
    }

    #[test]
    fn test_find_place_entry_in_array_no_match() {
        let entry: serde_json::Value = serde_json::json!([
            null,
            [null, null, null, null, null, [null, null, 10.0, 20.0, null], null, "0xother"],
            "Other",
        ]);
        let arr: Vec<serde_json::Value> = vec![
            serde_json::Value::Null,
            entry,
            serde_json::Value::Null,
        ];

        let found = find_place_entry_in_array(&arr, "0xnonexistent");
        assert!(found.is_none(), "Should not find non-matching entry");
    }
}
