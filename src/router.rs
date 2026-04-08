use std::time::Duration;

use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderName, Method},
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::{api, middleware as mw, state::SharedState};

/// Assemble the full application router.
///
/// Extracted from `main` so that integration tests can reuse the exact same
/// routing, middleware, and CORS configuration without duplicating it.
pub fn build_router(state: SharedState) -> Router {
    let max_upload = state.config.max_upload_bytes;

    let protected = Router::new()
        .route("/api/v1/process", post(api::process))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            mw::authenticate_and_rate_limit,
        ));

    Router::new()
        .route("/health", get(api::health))
        .route("/api/v1/palettes", get(api::list_palettes))
        .merge(protected)
        .with_state(state)
        .layer(DefaultBodyLimit::max(max_upload))
        .layer(TraceLayer::new_for_http())
        .layer(cors())
}

/// Build the CORS layer for the API.
///
/// - Origin:          any (API is public; RapidAPI proxies from various origins)
/// - Allowed methods: GET, POST, OPTIONS
/// - Allowed headers: Content-Type, X-Api-Key (our auth header), plus the two
///                    RapidAPI proxy headers forwarded on every request
/// - Exposed headers: X-RateLimit-* so browser/JS clients can read them
/// - Max age:         24 h to minimise preflight round-trips
fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            HeaderName::from_static("x-api-key"),
            HeaderName::from_static("x-rapidapi-key"),
            HeaderName::from_static("x-rapidapi-host"),
        ])
        .expose_headers([
            HeaderName::from_static("x-ratelimit-limit"),
            HeaderName::from_static("x-ratelimit-remaining"),
            HeaderName::from_static("x-ratelimit-reset"),
        ])
        .max_age(Duration::from_secs(86_400))
}
