use axum::{extract::State, response::{IntoResponse, Json}};
use serde::Serialize;

use crate::state::SharedState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status:          &'static str,
    pub version:         &'static str,
    pub environment:     String,
    pub palettes_loaded: usize,
    pub auth_enabled:    bool,
}

/// GET /health
///
/// Public endpoint. Returns server status, version, and runtime metadata.
/// Useful for load balancer health checks and uptime monitoring.
pub async fn health(State(state): State<SharedState>) -> impl IntoResponse {
    Json(HealthResponse {
        status:          "ok",
        version:         env!("CARGO_PKG_VERSION"),
        environment:     format!("{:?}", state.config.environment).to_lowercase(),
        palettes_loaded: state.palettes.len(),
        auth_enabled:    state.config.auth_enabled(),
    })
}
