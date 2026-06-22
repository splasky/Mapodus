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

use std::fmt;

use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct CookieAuth {
    pub session_id: String,
    pub csrf_token: String,
}

impl CookieAuth {
    pub fn from_cookie_str(cookie_str: &str) -> Result<Self> {
        let mut session_id = String::new();
        let mut csrf_token = String::new();

        for pair in cookie_str.split(';') {
            let trimmed = pair.trim();
            if let Some((key, value)) = trimmed.split_once('=') {
                match key {
                    "sessionid" => session_id = value.to_string(),
                    "csrftoken" => csrf_token = value.to_string(),
                    _ => (),
                }
            }
        }

        if session_id.is_empty() || csrf_token.is_empty() {
            return Err(anyhow!("Missing sessionid or csrftoken in cookie string"));
        }

        Ok(CookieAuth {
            session_id,
            csrf_token,
        })
    }

    pub fn to_cookie_header(&self) -> String {
        format!("sessionid={}; csrftoken={}", self.session_id, self.csrf_token)
    }

    pub fn to_csrf_header(&self) -> String {
        self.csrf_token.clone()
    }
}

impl fmt::Display for CookieAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CookieAuth {{ sessionid: {}, csrftoken: {} }}",
            self.session_id, self.csrf_token
        )
    }
}
