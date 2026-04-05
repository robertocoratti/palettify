use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    NotFound(String),

    #[error("missing or invalid API key")]
    Unauthorized,

    #[error("rate limit exceeded")]
    TooManyRequests,

    #[error("{0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "missing or invalid API key".to_string(),
            ),
            AppError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate limit exceeded — slow down and retry".to_string(),
            ),
            AppError::Internal(msg) => {
                tracing::error!("{}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "an internal error occurred".to_string(),
                )
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    fn status_of(e: AppError) -> StatusCode {
        let resp = e.into_response();
        resp.status()
    }

    #[test]
    fn bad_request_maps_to_400() {
        assert_eq!(status_of(AppError::BadRequest("oops".into())), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn not_found_maps_to_404() {
        assert_eq!(status_of(AppError::NotFound("gone".into())), StatusCode::NOT_FOUND);
    }

    #[test]
    fn unauthorized_maps_to_401() {
        assert_eq!(status_of(AppError::Unauthorized), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn too_many_requests_maps_to_429() {
        assert_eq!(status_of(AppError::TooManyRequests), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn internal_maps_to_500() {
        assert_eq!(
            status_of(AppError::Internal("boom".into())),
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    }
}
