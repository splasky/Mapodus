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

//! Application error types and related implementations.
//!
//! This module defines the `AppError` enum which represents various error
//! conditions that can occur in the application, along with necessary
//! trait implementations for error handling.

use std::fmt::{self, Display};
use std::error::Error;

/// Application-level errors that can occur during execution.
///
/// This enum provides a unified way to handle different types of errors
/// that might arise in the application, including HTTP errors, parsing
/// errors, I/O errors, and configuration errors.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppError {
    /// HTTP-related errors with descriptive message.
    Http(String),
    /// Parsing errors with descriptive message.
    Parse(String),
    /// I/O errors with descriptive message.
    Io(String),
    /// Configuration errors with descriptive message.
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
