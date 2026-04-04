// Shared helpers used by every integration test module.
//
// Each integration test file in tests/ declares `mod common;` to pull this in.
// Rust resolves that to tests/common/mod.rs automatically.

use axum::{
    body::Body,
    extract::DefaultBodyLimit,
    http::Request,
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
    };

    AppState::new(config)
}

/// Build a Router wired identically to main.rs but without a TCP listener.
/// Use with `tower::ServiceExt::oneshot` to send a single test request.
pub fn test_app(state: SharedState) -> Router {
    let max_upload = state.config.max_upload_bytes;

    let protected = Router::new()
        .route("/api/v1/process", post(api::process))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            mw::require_api_key,
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

/// A hardcoded 1x1 RGB PNG used as a minimal valid image in process tests.
pub fn minimal_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
        0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR chunk length + type
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // width=1, height=1
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // bit depth=8, color type=2 (RGB)
        0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, // IHDR CRC + IDAT length + type
        0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, // compressed pixel data
        0x00, 0x00, 0x02, 0x00, 0x01, 0xe2, 0x21, 0xbc, // IDAT data + CRC
        0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, // IDAT CRC + IEND
        0x44, 0xae, 0x42, 0x60, 0x82,                   // IEND CRC
    ]
}

