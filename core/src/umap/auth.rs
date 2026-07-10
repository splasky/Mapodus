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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_cookie_str_parses_normal_format() {
        let auth = CookieAuth::from_cookie_str("sessionid=abc123; csrftoken=xyz789").unwrap();
        assert_eq!(auth.session_id, "abc123");
        assert_eq!(auth.csrf_token, "xyz789");
    }

    #[test]
    fn from_cookie_str_handles_extra_whitespace() {
        let auth = CookieAuth::from_cookie_str("  sessionid=abc123 ;  csrftoken=xyz789  ").unwrap();
        assert_eq!(auth.session_id, "abc123");
        assert_eq!(auth.csrf_token, "xyz789");
    }

    #[test]
    fn from_cookie_str_ignores_extra_cookies() {
        let auth = CookieAuth::from_cookie_str(
            "sessionid=abc123; extra=ignored; csrftoken=xyz789; other=stuff",
        )
        .unwrap();
        assert_eq!(auth.session_id, "abc123");
        assert_eq!(auth.csrf_token, "xyz789");
    }

    #[test]
    fn from_cookie_str_fails_without_session_id() {
        let result = CookieAuth::from_cookie_str("csrftoken=xyz789");
        assert!(result.is_err());
    }

    #[test]
    fn from_cookie_str_fails_without_csrf_token() {
        let result = CookieAuth::from_cookie_str("sessionid=abc123");
        assert!(result.is_err());
    }

    #[test]
    fn from_cookie_str_fails_on_empty_string() {
        let result = CookieAuth::from_cookie_str("");
        assert!(result.is_err());
    }

    #[test]
    fn to_cookie_header_formats_correctly() {
        let auth = CookieAuth {
            session_id: "abc123".to_string(),
            csrf_token: "xyz789".to_string(),
        };
        assert_eq!(
            auth.to_cookie_header(),
            "sessionid=abc123; csrftoken=xyz789"
        );
    }

    #[test]
    fn to_csrf_header_returns_csrf_token() {
        let auth = CookieAuth {
            session_id: "abc123".to_string(),
            csrf_token: "xyz789".to_string(),
        };
        assert_eq!(auth.to_csrf_header(), "xyz789");
    }

    #[test]
    fn display_formats_correctly() {
        let auth = CookieAuth {
            session_id: "abc123".to_string(),
            csrf_token: "xyz789".to_string(),
        };
        assert_eq!(
            auth.to_string(),
            "CookieAuth { sessionid: abc123, csrftoken: xyz789 }"
        );
    }
}
