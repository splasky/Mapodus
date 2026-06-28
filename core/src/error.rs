use std::fmt::{self, Display};
use std::error::Error;

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
