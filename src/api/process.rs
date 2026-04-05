use axum::{
    extract::{Multipart, State},
    response::Response,
};

use crate::{error::AppError, processor::process_image, state::SharedState};

use super::{
    extract::parse_process_multipart,
    output::build_image_response,
};

/// POST /api/v1/process
///
/// Protected endpoint (requires X-Api-Key when auth is enabled).
///
/// Accepts multipart/form-data:
///   image        (required) - source image file (PNG, JPEG, WebP, BMP, ...)
///   palette_name (optional) - name of a built-in palette
///   palette      (optional) - newline-separated hex colors (#RRGGBB), one per line
///   format       (optional) - output format: png (default) | jpg | webp
///   algorithm    (optional) - nearest (default) | floyd-steinberg | ordered
///
/// Exactly one of palette_name or palette must be provided.
/// Returns the processed image with the appropriate Content-Type header.
pub async fn process(
    State(state): State<SharedState>,
    mut mp:       Multipart,
) -> Result<Response, AppError> {
    let input = parse_process_multipart(&mut mp, &state).await?;

    let img = image::load_from_memory(&input.image_bytes)
        .map_err(|e| AppError::BadRequest(format!("could not decode image: {e}")))?;

    let result = tokio::task::spawn_blocking(move || {
        process_image(img, &input.palette, input.algorithm)
    })
    .await
    .map_err(|e| AppError::Internal(format!("processing task panicked: {e}")))?;

    build_image_response(result, input.format)
}
