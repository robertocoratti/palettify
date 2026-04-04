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
#[path = "tests/error.rs"]
mod tests;
