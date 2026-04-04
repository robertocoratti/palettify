use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{error::AppError, state::SharedState};

/// Axum middleware that enforces API key authentication.
///
/// Reads the `X-Api-Key` header and validates it against the set of keys in
/// Config. If Config.api_keys is None (auth disabled), all requests pass
/// through unchanged — useful for local development without any configuration.
///
/// Returns 401 Unauthorized if the header is missing or the key is not in the
/// allowed set.
pub async fn require_api_key(
    State(state): State<SharedState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if let Some(api_keys) = &state.config.api_keys {
        let provided = request
            .headers()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        if !api_keys.contains(provided) {
            return Err(AppError::Unauthorized);
        }
    }

    Ok(next.run(request).await)
}
