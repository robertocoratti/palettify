use axum::extract::Multipart;

use crate::{
    algorithm::Algorithm,
    error::AppError,
    palette::Palette,
    state::AppState,
};

use super::output::OutputFormat;

/// All inputs for a `/process` request, fully validated and resolved.
pub struct ProcessInput {
    pub image_bytes: Vec<u8>,
    pub palette:     Palette,
    pub algorithm:   Algorithm,
    pub format:      OutputFormat,
}

/// Parse and validate every multipart field from a `/process` request,
/// resolving the palette from either a built-in name or raw hex colors.
pub async fn parse_process_multipart(
    mp:    &mut Multipart,
    state: &AppState,
) -> Result<ProcessInput, AppError> {
    let mut image_bytes:    Option<Vec<u8>>    = None;
    let mut palette_name:   Option<String>     = None;
    let mut palette_colors: Option<Vec<String>> = None;
    let mut format    = OutputFormat::default();
    let mut algorithm = Algorithm::default();

    while let Ok(Some(field)) = mp.next_field().await {
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
                    format = OutputFormat::from_str(&fmt);
                }
            }
            "algorithm" => {
                let val = field
                    .text()
                    .await
                    .map_err(|_| AppError::BadRequest("could not read algorithm field".into()))?;

                algorithm = Algorithm::from_str(&val).ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "unknown algorithm '{}' — valid values: nearest, floyd-steinberg, ordered",
                        val.trim()
                    ))
                })?;
            }
            _ => {}
        }
    }

    let palette = resolve_palette(palette_name, palette_colors, state)?;

    let image_bytes = image_bytes
        .filter(|b| !b.is_empty())
        .ok_or_else(|| AppError::BadRequest("missing or empty 'image' field".into()))?;

    Ok(ProcessInput { image_bytes, palette, algorithm, format })
}

/// Resolve the palette from either a built-in name or a raw list of hex colors.
fn resolve_palette(
    name:   Option<String>,
    colors: Option<Vec<String>>,
    state:  &AppState,
) -> Result<Palette, AppError> {
    match (name, colors) {
        (Some(name), _) => state
            .palettes
            .get(&name)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!(
                "palette '{name}' not found — use GET /api/v1/palettes for available options"
            ))),
        (None, Some(colors)) => {
            let refs: Vec<&str> = colors.iter().map(String::as_str).collect();
            Palette::from_hex_list("custom", &refs)
        }
        (None, None) => Err(AppError::BadRequest(
            "provide either 'palette_name' (built-in) or 'palette' (newline-separated hex colors)".into(),
        )),
    }
}
