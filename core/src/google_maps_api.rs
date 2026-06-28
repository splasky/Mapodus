use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleList {
    pub id: String,
    pub title: String,
    pub place_count: u32,
}

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

struct ProtoWriter {
    buf: Vec<u8>,
}

impl ProtoWriter {
    fn new() -> Self {
        ProtoWriter { buf: Vec::new() }
    }

    fn write_varint(&mut self, value: u64) {
        let mut v = value;
        loop {
            if v < 0x80 {
                self.buf.push(v as u8);
                break;
            }
            self.buf.push((v as u8 & 0x7F) | 0x80);
            v >>= 7;
        }
    }

    fn write_tag(&mut self, field: u32, wire_type: u32) {
        self.write_varint(((field << 3) | wire_type) as u64);
    }

    fn write_string(&mut self, field: u32, value: &str) {
        self.write_tag(field, 2);
        self.write_varint(value.len() as u64);
        self.buf.extend_from_slice(value.as_bytes());
    }

    fn write_bool(&mut self, field: u32, value: bool) {
        self.write_tag(field, 0);
        self.write_varint(if value { 1 } else { 0 });
    }

    fn write_uint64(&mut self, field: u32, value: u64) {
        self.write_tag(field, 0);
        self.write_varint(value);
    }

    fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

fn build_mas_request() -> Vec<u8> {
    let mut w = ProtoWriter::new();
    w.write_string(1, "en");
    w.write_string(2, "US");
    w.into_bytes()
}

fn build_getlist_request(list_id: &str, session_token: &str) -> Vec<u8> {
    let mut w = ProtoWriter::new();
    w.write_varint((1 << 3) | 0);
    w.write_varint(1);
    w.write_string(2, list_id);
    w.write_bool(3, false);
    w.write_bool(4, false);
    w.write_string(11, session_token);
    w.write_uint64(14, 1);
    w.into_bytes()
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

    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("X-Same-Domain", "1".parse().unwrap());

        let cookie_str = self
            .cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ");
        headers.insert(
            reqwest::header::COOKIE,
            cookie_str.parse().unwrap(),
        );
        headers
    }

    fn sapisid_hash(&self) -> Option<String> {
        let sapisid = self.cookies.get("SAPISID")
            .or_else(|| self.cookies.get("__Secure-1PSAPISID"))?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        let input = format!("{} {}", timestamp, sapisid);
        let hash = {
            let digest = md5(input.as_bytes());
            hex_encode(&digest)
        };
        Some(format!("SAPISIDHASH {}_{}", timestamp, hash))
    }

    pub async fn get_session_token(&self) -> Result<String, crate::error::AppError> {
        let url = "https://maps.google.com/";
        let mut headers = self.build_headers();
        if let Some(auth) = self.sapisid_hash() {
            headers.insert("Authorization", auth.parse().unwrap());
        }

        let response = self
            .client
            .get(url)
            .headers(headers)
            .send()
            .await?;
        let html = response.text().await?;

        let token = extract_session_token(&html)
            .ok_or_else(|| crate::error::AppError::Parse("Failed to extract session token from APP_OPTIONS".into()))?;
        Ok(token)
    }

    pub async fn get_all_lists(&self, _session_token: &str) -> Result<Vec<GoogleList>, crate::error::AppError> {
        let url = "https://www.google.com/locationhistory/preview/mas";
        let mut headers = self.build_headers();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/x-javascript".parse().unwrap(),
        );
        if let Some(auth) = self.sapisid_hash() {
            headers.insert("Authorization", auth.parse().unwrap());
        }

        let body = build_mas_request();
        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await?;
        let data = response.bytes().await?;

        let json_str = strip_garbage_prefix(&data);
        let parsed: serde_json::Value = serde_json::from_str(json_str)?;

        let lists = parse_lists_from_mas_response(&parsed)?;
        Ok(lists)
    }

    pub async fn get_list_places(
        &self,
        list_id: &str,
        session_token: &str,
    ) -> Result<Vec<GoogleSavedPlace>, crate::error::AppError> {
        let url = "https://www.google.com/maps/preview/entitylist/getlist";
        let mut headers = self.build_headers();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/x-javascript".parse().unwrap(),
        );
        if let Some(auth) = self.sapisid_hash() {
            headers.insert("Authorization", auth.parse().unwrap());
        }

        let body = build_getlist_request(list_id, session_token);
        let response = self
            .client
            .post(url)
            .headers(headers)
            .body(body)
            .send()
            .await?;
        let data = response.bytes().await?;

        let json_str = strip_garbage_prefix(&data);
        let parsed: serde_json::Value = serde_json::from_str(json_str)?;

        let places = parse_places_from_getlist_response(&parsed, list_id)?;
        Ok(places)
    }

    pub async fn collect_all(
        &self,
    ) -> Result<Vec<GoogleSavedPlace>, crate::error::AppError> {
        let session_token = self.get_session_token().await?;
        let lists = self.get_all_lists(&session_token).await?;

        let mut all_places = Vec::new();
        for list in &lists {
            match self.get_list_places(&list.id, &session_token).await {
                Ok(places) => all_places.extend(places),
                Err(e) => {
                    eprintln!("Warning: failed to fetch list '{}': {}", list.title, e);
                }
            }
        }
        Ok(all_places)
    }
}

fn strip_garbage_prefix(data: &[u8]) -> &str {
    let start = if data.len() > 4 && data[0] == b')' && data[1] == b']' {
        4
    } else {
        0
    };
    std::str::from_utf8(&data[start..]).unwrap_or("")
}

fn parse_lists_from_mas_response(value: &serde_json::Value) -> Result<Vec<GoogleList>, crate::error::AppError> {
    let mut lists = Vec::new();
    if let Some(entries) = value.get("entries").and_then(|v| v.as_array()) {
        for entry in entries {
            if let Some(list_data) = entry.get("1").and_then(|v| v.get("1")) {
                let id = list_data
                    .get("1")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let title = entry
                    .get("1")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                let place_count = entry
                    .get("4")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;

                if !id.is_empty() {
                    lists.push(GoogleList {
                        id,
                        title,
                        place_count,
                    });
                }
            }
        }
    }
    Ok(lists)
}

fn parse_places_from_getlist_response(
    value: &serde_json::Value,
    list_name: &str,
) -> Result<Vec<GoogleSavedPlace>, crate::error::AppError> {
    let mut places = Vec::new();
    if let Some(items) = value.get("1").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(place_data) = item.get("1") {
                let title = place_data
                    .get("1")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let address = place_data
                    .get("2")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let place_id = place_data
                    .get("3")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let latitude = place_data.get("4").and_then(|v| v.as_f64());
                let longitude = place_data.get("5").and_then(|v| v.as_f64());
                let url = place_data
                    .get("6")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let notes = place_data
                    .get("7")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                places.push(GoogleSavedPlace {
                    title,
                    address,
                    latitude,
                    longitude,
                    url,
                    notes,
                    place_id,
                    list: list_name.to_string(),
                });
            }
        }
    }
    Ok(places)
}

fn md5(input: &[u8]) -> [u8; 16] {
    Md5Context::digest(input)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

struct Md5Context;

impl Md5Context {
    fn digest(input: &[u8]) -> [u8; 16] {
        let mut ctx = Md5State::new();
        ctx.update(input);
        ctx.finalize()
    }
}

const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
    0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
    0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
    0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
    0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
    0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
    0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
    0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
    0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

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

fn extract_session_token(html: &str) -> Option<String> {
    let marker = "window.APP_OPTIONS";
    let start = html.find(marker)?;
    let after_marker = &html[start + marker.len()..];
    let bracket_start = after_marker.find('[')?;
    let depth = find_matching_bracket(after_marker, bracket_start)?;
    let array_str = &after_marker[bracket_start..=depth];

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(array_str) {
        if let Some(arr) = value.as_array() {
            if let Some(token) = arr.get(11) {
                if let Some(s) = token.as_str() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

fn find_matching_bracket(s: &str, open: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if open >= bytes.len() || bytes[open] != b'[' {
        return None;
    }
    let mut depth = 0u32;
    for i in open..bytes.len() {
        match bytes[i] {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
