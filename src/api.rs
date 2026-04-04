use axum::{
    body::Body,
    extract::{Multipart, State},
    http::{header, Response, StatusCode},
    response::{IntoResponse, Json},
};
use image::ImageFormat;
use serde::Serialize;
use serde_json::json;
use std::io::Cursor;

use crate::{
    error::AppError,
    palette::Palette,
    processor::process_image,
    state::SharedState,
};

// Response types

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub environment: String,
    pub palettes_loaded: usize,
    pub auth_enabled: bool,
}

#[derive(Serialize)]
pub struct PalettesResponse {
    pub total: usize,
    pub palettes: Vec<PaletteInfo>,
}

#[derive(Serialize)]
pub struct PaletteInfo {
    pub name: String,
    pub colors: usize,
    pub preview: Vec<String>,
}

// Handlers

/// GET /health
///
/// Public endpoint. Returns server status, version, and runtime metadata.
/// Useful for load balancer health checks and uptime monitoring.
pub async fn health(State(state): State<SharedState>) -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        environment: format!("{:?}", state.config.environment).to_lowercase(),
        palettes_loaded: state.palettes.len(),
        auth_enabled: state.config.auth_enabled(),
    })
}

/// GET /api/v1/palettes
///
/// Public endpoint. Lists all available palettes with color counts and
/// a short preview of the first five hex values.
pub async fn list_palettes(State(state): State<SharedState>) -> impl IntoResponse {
    let mut palettes: Vec<PaletteInfo> = state
        .palettes
        .values()
        .map(|p| PaletteInfo {
            name: p.name.clone(),
            colors: p.color_count(),
            preview: p.preview_colors(5),
        })
        .collect();

    palettes.sort_by(|a, b| a.name.cmp(&b.name));

    Json(PalettesResponse {
        total: palettes.len(),
        palettes,
    })
}

/// POST /api/v1/process
///
/// Protected endpoint (requires X-Api-Key when auth is enabled).
///
/// Accepts multipart/form-data:
///   image        (required) - source image file (PNG, JPEG, WebP, BMP, ...)
///   palette_name (optional) - name of a built-in palette
///   palette      (optional) - newline-separated hex colors (#RRGGBB), one per line
///   format       (optional) - output format: png (default) | jpg | webp
///
/// Exactly one of palette_name or palette must be provided.
/// Returns the processed image with the appropriate Content-Type header.
pub async fn process(
    State(state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<axum::response::Response, AppError> {
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut palette_name: Option<String> = None;
    let mut palette_colors: Option<Vec<String>> = None;
    let mut output_format = "png".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name().unwrap_or("") {
            "image" => {
                image_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| AppError::BadRequest(format!("could not read image field: {e}")))?
                        .to_vec(),
                );
            }
            "palette_name" => {
                palette_name = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::BadRequest("could not read palette_name field".into()))?
                        .trim()
                        .to_string(),
                );
            }
            "palette" => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| AppError::BadRequest("could not read palette field".into()))?;

                let colors: Vec<String> = text
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(String::from)
                    .collect();

                if !colors.is_empty() {
                    palette_colors = Some(colors);
                }
            }
            "format" => {
                if let Ok(fmt) = field.text().await {
                    output_format = fmt.trim().to_lowercase();
                }
            }
            _ => {}
        }
    }

    // Resolve palette from either a name (built-in or loaded from disk)
    // or a raw list of hex colors supplied in the request.
    let palette = match (palette_name, palette_colors) {
        (Some(name), _) => {
            state
                .palettes
                .get(&name)
                .cloned()
                .ok_or_else(|| AppError::NotFound(
                    format!("palette '{name}' not found — use GET /api/v1/palettes for available options")
                ))?
        }
        (None, Some(colors)) => {
            let refs: Vec<&str> = colors.iter().map(String::as_str).collect();
            Palette::from_hex_list("custom", &refs)?
        }
        (None, None) => {
            return Err(AppError::BadRequest(
                "provide either 'palette_name' (built-in) or 'palette' (newline-separated hex colors)".into(),
            ));
        }
    };

    // Validate and decode the source image.
    let bytes = image_bytes
        .filter(|b| !b.is_empty())
        .ok_or_else(|| AppError::BadRequest("missing or empty 'image' field".into()))?;

    let img = image::load_from_memory(&bytes)
        .map_err(|e| AppError::BadRequest(format!("could not decode image: {e}")))?;

    // Run the CPU-bound processing in a dedicated blocking thread so the
    // async runtime is not blocked during image remapping.
    let result = tokio::task::spawn_blocking(move || process_image(img, &palette))
        .await
        .map_err(|e| AppError::Internal(format!("processing task panicked: {e}")))?;

    // Encode the output image.
    let (fmt, content_type, ext) = match output_format.as_str() {
        "jpg" | "jpeg" => (ImageFormat::Jpeg, "image/jpeg", "jpg"),
        "webp"         => (ImageFormat::WebP,  "image/webp",  "webp"),
        _              => (ImageFormat::Png,   "image/png",   "png"),
    };

    let dynamic = image::DynamicImage::ImageRgb8(result);
    let mut buf = Cursor::new(Vec::new());
    dynamic
        .write_to(&mut buf, fmt)
        .map_err(|e| AppError::Internal(format!("failed to encode output image: {e}")))?;

    let image_bytes = buf.into_inner();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, image_bytes.len())
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"result.{ext}\""),
        )
        .body(Body::from(image_bytes))
        .unwrap())
}

/// Convenience wrapper used in tests to build a minimal JSON error response.
#[allow(dead_code)]
pub fn error_json(status: StatusCode, message: &str) -> axum::response::Response {
    (status, Json(json!({ "error": message }))).into_response()
}
