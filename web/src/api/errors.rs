use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
#[allow(dead_code)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, Json(json!({"error": msg}))).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::BadRequest(e.to_string())
    }
}

impl From<tower_sessions::session::Error> for ApiError {
    fn from(e: tower_sessions::session::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}

impl From<umap_core::error::AppError> for ApiError {
    fn from(e: umap_core::error::AppError) -> Self {
        ApiError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn check_response(err: ApiError, expected_status: StatusCode) {
        let response = err.into_response();
        assert_eq!(response.status(), expected_status);
    }

    #[test]
    fn bad_request_returns_400() {
        let err = ApiError::BadRequest("bad input".to_string());
        check_response(err, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn unauthorized_returns_401() {
        let err = ApiError::Unauthorized("login required".to_string());
        check_response(err, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn not_found_returns_404() {
        let err = ApiError::NotFound("missing".to_string());
        check_response(err, StatusCode::NOT_FOUND);
    }

    #[test]
    fn internal_error_returns_500() {
        let err = ApiError::Internal("server error".to_string());
        check_response(err, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn converts_from_anyhow_to_internal() {
        let err = ApiError::from(anyhow::anyhow!("oops"));
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn converts_from_serde_json_to_bad_request() {
        let inner = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let err = ApiError::from(inner);
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
