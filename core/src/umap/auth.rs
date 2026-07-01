use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        format!(
            "sessionid={}; csrftoken={}",
            self.session_id, self.csrf_token
        )
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
