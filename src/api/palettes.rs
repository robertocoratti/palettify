use axum::{extract::State, response::{IntoResponse, Json}};
use serde::Serialize;

use crate::state::SharedState;

#[derive(Serialize)]
pub struct PaletteInfo {
    pub name:    String,
    pub colors:  usize,
    pub preview: Vec<String>,
}

#[derive(Serialize)]
pub struct PalettesResponse {
    pub total:    usize,
    pub palettes: Vec<PaletteInfo>,
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
            name:    p.name.clone(),
            colors:  p.color_count(),
            preview: p.preview_colors(5),
        })
        .collect();

    palettes.sort_by(|a, b| a.name.cmp(&b.name));

    Json(PalettesResponse {
        total: palettes.len(),
        palettes,
    })
}
