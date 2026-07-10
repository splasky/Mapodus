use std::error::Error;
use std::fmt::{self, Display};

#[derive(Debug, Clone)]
pub enum AppError {
    Http(String),
    Parse(String),
    Io(String),
    Config(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Http(msg) => write!(f, "HTTP error: {}", msg),
            AppError::Parse(msg) => write!(f, "Parse error: {}", msg),
            AppError::Io(msg) => write!(f, "I/O error: {}", msg),
            AppError::Config(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl Error for AppError {}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError::Config(error.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        AppError::Http(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::Parse(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_http_error() {
        let err = AppError::Http("connection failed".to_string());
        assert_eq!(err.to_string(), "HTTP error: connection failed");
    }

    #[test]
    fn displays_parse_error() {
        let err = AppError::Parse("invalid json".to_string());
        assert_eq!(err.to_string(), "Parse error: invalid json");
    }

    #[test]
    fn displays_io_error() {
        let err = AppError::Io("file not found".to_string());
        assert_eq!(err.to_string(), "I/O error: file not found");
    }

    #[test]
    fn displays_config_error() {
        let err = AppError::Config("missing setting".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing setting");
    }

    #[test]
    fn converts_from_anyhow_error_to_config() {
        let inner = anyhow::anyhow!("something went wrong");
        let err = AppError::from(inner);
        assert!(matches!(err, AppError::Config(_)));
        assert!(err.to_string().contains("something went wrong"));
    }

    #[test]
    fn http_variant_display() {
        let err = AppError::Http("connection refused".to_string());
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn converts_from_serde_json_error_to_parse() {
        let inner = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let err = AppError::from(inner);
        assert!(matches!(err, AppError::Parse(_)));
    }
}
