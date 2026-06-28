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

    fn auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("X-Same-Domain", "1".parse().unwrap());
        headers.insert(
            reqwest::header::COOKIE,
            self.cookie_string().parse().unwrap(),
        );
        if let Some(auth) = self.sapisid_hash() {
            headers.insert("Authorization", auth.parse().unwrap());
        }
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
        let hash = hex_encode(&md5(input.as_bytes()));
        Some(format!("SAPISIDHASH {}_{}", timestamp, hash))
    }

    pub async fn get_all_saved_places(&self) -> Result<Vec<GoogleSavedPlace>, crate::error::AppError> {
        let url = "https://www.google.com/maps/preview/entitylist/getlist";
        let headers = self.auth_headers();

        let response = self
            .client
            .get(url)
            .query(&[
                ("authuser", "0"),
                ("hl", "en"),
                ("gl", "us"),
            ])
            .headers(headers)
            .send()
            .await?;

        let status = response.status();
        let data = response.bytes().await?;

        if !status.is_success() {
            let preview = String::from_utf8_lossy(&data[..data.len().min(200)]);
            return Err(crate::error::AppError::Http(format!(
                "getlist endpoint returned {}: {}",
                status, preview
            )));
        }

        let json_str = strip_xssi(&data).ok_or_else(|| {
            let preview = String::from_utf8_lossy(&data[..data.len().min(500)]);
            crate::error::AppError::Parse(format!(
                "Response is not valid UTF-8. Body preview: {}",
                preview
            ))
        })?;

        let parsed: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
            let preview = if json_str.len() > 500 { &json_str[..500] } else { json_str };
            crate::error::AppError::Parse(format!(
                "JSON parse error: {}. Body: {}",
                e, preview
            ))
        })?;

        let places = parse_saved_response(&parsed)?;
        Ok(places)
    }
}

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

fn parse_saved_response(value: &serde_json::Value) -> Result<Vec<GoogleSavedPlace>, crate::error::AppError> {
    let mut places = Vec::new();

    // Response structure: [ [ list_name, ... entries ..., [ [ place, ... ] ] ] ]
    // From the Firefox extension and gmaps-list research:
    //   root[4] = list name
    //   root[8] = array of place entries
    //   Each entry: [ place_info, name, comment, ... ]
    //     entry[2] = name
    //     entry[3] = comment/notes
    //     entry[1] = place_info array:
    //       [ _, address?, _, _, _, [_, _, lat, lng], _, place_id, ... ]
    //       place_info[2] = address (sometimes)
    //       place_info[5] = [_, _, lat, lng]
    //       place_info[7] = place_id

    let root = value.as_array().and_then(|a| a.first())
        .and_then(|v| v.as_array());

    let root = match root {
        Some(r) => r,
        None => return Ok(places),
    };

    let list_name = root.get(4)
        .and_then(|v| v.as_str())
        .unwrap_or("Imported")
        .to_string();

    let entries = root.get(8).and_then(|v| v.as_array());

    if let Some(items) = entries {
        for item in items {
            let arr = match item.as_array() {
                Some(a) => a,
                None => continue,
            };

            let name = arr.get(2).and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() {
                continue;
            }

            let place_info = arr.get(1).and_then(|v| v.as_array());

            let address = place_info.and_then(|pi| {
                pi.get(2).and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .or_else(|| pi.get(4).and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
            }).map(|s| s.to_string());

            let coords = place_info.and_then(|pi| {
                pi.get(5).and_then(|v| v.as_array())
            });

            let latitude = coords.and_then(|c| c.get(2).and_then(|v| v.as_f64()));
            let longitude = coords.and_then(|c| c.get(3).and_then(|v| v.as_f64()));

            let place_id = place_info
                .and_then(|pi| pi.get(7).and_then(|v| v.as_str()))
                .map(|s| s.to_string());

            let url = place_id.as_ref().map(|id| {
                format!("https://www.google.com/maps/place/?q=place_id:{}", id)
            });

            let notes = arr.get(3).and_then(|v| v.as_str())
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
                list: list_name.clone(),
            });
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
