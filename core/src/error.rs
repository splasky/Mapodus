// Copyright 2026 HYChang
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
