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
        headers.insert(
            "x-maps-diversion-context-bin",
            "CAE=".parse().unwrap(),
        );
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

    fn sapisid_hash(&self) -> Option<String> {
        let sapisid = self
            .cookies
            .get("SAPISID")
            .or_else(|| self.cookies.get("__Secure-3PAPISID"))
            .or_else(|| self.cookies.get("__Secure-1PSAPISID"))?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        let input = format!("{} {}", timestamp, sapisid);
        let hash = hex_encode(&md5(input.as_bytes()));
        Some(format!("SAPISIDHASH {}_{}", timestamp, hash))
    }

    /// Get all saved lists via the MAS endpoint.
    pub async fn get_all_lists(&self) -> Result<Vec<GoogleSavedList>, crate::error::AppError> {
        let pb = build_mas_pb();
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
            return Err(crate::error::AppError::Http(format!(
                "MAS endpoint returned {}: {}",
                status, preview
            )));
        }
        let body = strip_xssi(&data).ok_or_else(|| {
            crate::error::AppError::Parse("MAS response is not valid UTF-8".into())
        })?;
        let json: serde_json::Value = serde_json::from_str(body).map_err(|e| {
            crate::error::AppError::Parse(format!(
                "MAS JSON parse error: {}. Body (first 500): {}",
                e,
                &body[..body.len().min(500)]
            ))
        })?;
        parse_mas_lists(&json)
    }

    /// Get places for a specific list using its placelists page.
    pub async fn get_list_places(
        &self,
        list_id: &str,
        list_name: &str,
    ) -> Result<Vec<GoogleSavedPlace>, crate::error::AppError> {
        let url = format!(
            "https://www.google.com/maps/preview/entitylist/getlist?authuser=0&hl=en&gl=us&pb=!1m2!1s{}!2s0",
            list_id
        );
        let response = self
            .client
            .get(&url)
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
        let body = strip_xssi(&data).ok_or_else(|| {
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
        parse_getlist_places(&json, list_name)
    }

    /// High-level: get all places from all saved lists.
    pub async fn get_all_saved_places(&self) -> Result<Vec<GoogleSavedPlace>, crate::error::AppError> {
        let lists = self.get_all_lists().await?;
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
            match self.get_list_places(&list.id, &list.name).await {
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

    fn into_string(self) -> String {
        self.buf
    }
}

/// Write a message field with its token count prefix.
/// The format is `!{field}m{token_count_of_children}{children_encoded}`.
/// token_count includes all descendants.
/// Build the pb parameter for the MAS endpoint.
fn build_mas_pb() -> String {
    // Minimal known-working format from real curl request:
    // !2m3!1s{token}!7e81!15i20393!7m1!1i50!9m0!12m1!1i50!15m1!1i50
    let mut w = ProtoTextWriter::new();

    // Field 2: message with auth/version info
    let mut inner = ProtoTextWriter::new();
    inner.write_string(1, "A");
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

    w.into_string()
}

// ── Response parsing ──

fn strip_xssi(data: &[u8]) -> Option<&str> {
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
fn parse_mas_lists(value: &serde_json::Value) -> Result<Vec<GoogleSavedList>, crate::error::AppError> {
    let root = value.as_array().ok_or_else(|| {
        crate::error::AppError::Parse("MAS response root is not an array".into())
    })?;

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
            let url = arr.get(2).and_then(|v| v.as_array())
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
        if arr[0].is_null() && arr[1].is_null() && arr[2].is_null() {
            if let Some(entries) = arr[3].as_array() {
                if !entries.is_empty() {
                    // Verify at least one entry looks like a saved list
                    if entries.iter().any(|e| looks_like_saved_list(e)) {
                        return Some(entries);
                    }
                }
            }
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
        if arr.iter().all(|e| e.is_array()) {
            if let Some(first) = arr.first().and_then(|v| v.as_array()) {
                if first.len() >= 5 && first[0].is_string() {
                    return Some(arr);
                }
            }
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
    let root = match value.as_array().and_then(|a| a.first()).and_then(|v| v.as_array()) {
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
        let url = place_id.as_ref().map(|id| {
            format!("https://www.google.com/maps/place/?q=place_id:{}", id)
        });
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

// ── MD5 (no external dependency) ──

fn md5(input: &[u8]) -> [u8; 16] {
    Md5Context::digest(input)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
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
            b = b.wrapping_add(a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g]).rotate_left(S[i]));
            a = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
}
