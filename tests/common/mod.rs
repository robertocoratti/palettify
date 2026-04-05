// Shared helpers used by every integration test module.
//
// Each integration test file in tests/ declares `mod common;` to pull this in.
// Rust resolves that to tests/common/mod.rs automatically.

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
    Router,
};
use serde_json::Value;
use std::collections::HashSet;

use palettify::{
    api,
    config::{Config, Environment},
    middleware as mw,
    state::{AppState, SharedState},
};

/// Build an in-memory application state without touching the filesystem.
/// Pass `Some(vec!["key"])` to enable API key auth; `None` to disable it.
pub fn test_state(api_keys: Option<Vec<&str>>) -> SharedState {
    let api_keys =
        api_keys.map(|keys| keys.into_iter().map(String::from).collect::<HashSet<_>>());

    let config = Config {
        port: 3000,
        environment: Environment::Development,
        palettes_dir: None,
        api_keys,
        max_upload_bytes: 10 * 1024 * 1024,
        upstash_rest_url: None,
        upstash_rest_token: None,
        rate_limit_requests: 60,
        rate_limit_window_secs: 60,
    };

    // No Redis in tests — rate limiting and usage tracking are disabled.
    AppState::new(config, None)
}

/// Build a Router wired identically to main.rs but without a TCP listener.
/// Use with `tower::ServiceExt::oneshot` to send a single test request.
pub fn test_app(state: SharedState) -> Router {
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
}

/// Consume a response body and parse it as JSON.
pub async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Build a minimal valid multipart/form-data body containing the given PNG
/// bytes, an optional built-in palette name, and an optional output format.
/// Returns the Content-Type header value and the raw body bytes.
pub fn make_multipart_body(
    image_bytes: &[u8],
    palette_name: Option<&str>,
    format: Option<&str>,
) -> (String, Vec<u8>) {
    let boundary = "boundary-test-1234";
    let mut body = Vec::new();

    let push = |buf: &mut Vec<u8>, s: &str| buf.extend_from_slice(s.as_bytes());

    push(&mut body, &format!("--{boundary}\r\n"));
    push(
        &mut body,
        "Content-Disposition: form-data; name=\"image\"; filename=\"test.png\"\r\n",
    );
    push(&mut body, "Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(image_bytes);
    push(&mut body, "\r\n");

    if let Some(name) = palette_name {
        push(&mut body, &format!("--{boundary}\r\n"));
        push(
            &mut body,
            "Content-Disposition: form-data; name=\"palette_name\"\r\n\r\n",
        );
        push(&mut body, name);
        push(&mut body, "\r\n");
    }

    if let Some(fmt) = format {
        push(&mut body, &format!("--{boundary}\r\n"));
        push(&mut body, "Content-Disposition: form-data; name=\"format\"\r\n\r\n");
        push(&mut body, fmt);
        push(&mut body, "\r\n");
    }

    push(&mut body, &format!("--{boundary}--\r\n"));

    (format!("multipart/form-data; boundary={boundary}"), body)
}

/// Generates a valid 1×1 red RGB PNG using the image crate.
/// Avoids hand-rolled byte arrays that are brittle across decoder versions.
pub fn minimal_png() -> Vec<u8> {
    use image::{ImageBuffer, ImageFormat, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(1, 1, Rgb([255u8, 0u8, 0u8]));
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Png).unwrap();
    buf.into_inner()
}

/// Extended multipart builder that supports all process-endpoint fields.
///
/// - `palette_name`  — built-in palette slug
/// - `palette_hex`   — newline-separated `#RRGGBB` colors (custom palette)
/// - `format`        — output format: `png` | `jpg` | `webp`
/// - `algorithm`     — `nearest` | `floyd-steinberg` | `ordered`
pub fn make_multipart_full(
    image_bytes:  &[u8],
    palette_name: Option<&str>,
    palette_hex:  Option<&str>,
    format:       Option<&str>,
    algorithm:    Option<&str>,
) -> (String, Vec<u8>) {
    let boundary = "boundary-test-full-1234";
    let mut body = Vec::new();
    let push = |buf: &mut Vec<u8>, s: &str| buf.extend_from_slice(s.as_bytes());

    push(&mut body, &format!("--{boundary}\r\n"));
    push(&mut body, "Content-Disposition: form-data; name=\"image\"; filename=\"test.png\"\r\n");
    push(&mut body, "Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(image_bytes);
    push(&mut body, "\r\n");

    if let Some(name) = palette_name {
        push(&mut body, &format!("--{boundary}\r\n"));
        push(&mut body, "Content-Disposition: form-data; name=\"palette_name\"\r\n\r\n");
        push(&mut body, name);
        push(&mut body, "\r\n");
    }

    if let Some(hex) = palette_hex {
        push(&mut body, &format!("--{boundary}\r\n"));
        push(&mut body, "Content-Disposition: form-data; name=\"palette\"\r\n\r\n");
        push(&mut body, hex);
        push(&mut body, "\r\n");
    }

    if let Some(fmt) = format {
        push(&mut body, &format!("--{boundary}\r\n"));
        push(&mut body, "Content-Disposition: form-data; name=\"format\"\r\n\r\n");
        push(&mut body, fmt);
        push(&mut body, "\r\n");
    }

    if let Some(alg) = algorithm {
        push(&mut body, &format!("--{boundary}\r\n"));
        push(&mut body, "Content-Disposition: form-data; name=\"algorithm\"\r\n\r\n");
        push(&mut body, alg);
        push(&mut body, "\r\n");
    }

    push(&mut body, &format!("--{boundary}--\r\n"));
    (format!("multipart/form-data; boundary={boundary}"), body)
}

/// Build a state with an active Upstash client pointing at `redis_url`.
/// Use this for rate-limiting integration tests.
pub fn test_state_with_redis(api_keys: Option<Vec<&str>>, redis_url: &str) -> SharedState {
    let api_keys =
        api_keys.map(|keys| keys.into_iter().map(String::from).collect::<HashSet<_>>());

    let config = Config {
        port:                  3000,
        environment:           Environment::Development,
        palettes_dir:          None,
        api_keys,
        max_upload_bytes:      10 * 1024 * 1024,
        upstash_rest_url:      Some(redis_url.to_string()),
        upstash_rest_token:    Some("test-token".to_string()),
        rate_limit_requests:   10,
        rate_limit_window_secs: 60,
    };

    let redis = Some(palettify::upstash::UpstashClient::new(
        redis_url.to_string(),
        "test-token".to_string(),
    ));

    AppState::new(config, redis)
}

