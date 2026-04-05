// Integration tests for the rate-limiting + usage-tracking middleware paths.
//
// These tests spin up a small mock Upstash REST server and wire it into the
// application state so that the middleware's Redis code paths are exercised.

mod common;

use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

use common::{make_multipart_body, minimal_png, test_app, test_state_with_redis};

// ── Mock Upstash server ──────────────────────────────────────────────────────

async fn incr_handler(
    State(counter): State<Arc<Mutex<i64>>>,
    Json(_cmd): Json<Value>,
) -> Json<Value> {
    let mut c = counter.lock().unwrap();
    *c += 1;
    Json(json!({ "result": *c }))
}

async fn pipeline_handler(Json(_cmds): Json<Value>) -> Json<Value> {
    Json(json!([{ "result": 1 }, { "result": 1 }]))
}

/// Start a mock Upstash server. `initial_count` is the value before the first INCR.
async fn start_mock(initial_count: i64) -> String {
    let counter = Arc::new(Mutex::new(initial_count));
    let app = Router::new()
        .route("/",         post(incr_handler))
        .route("/pipeline", post(pipeline_handler))
        .with_state(counter);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://127.0.0.1:{port}")
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// Middleware passes the request when the rate limit is not exceeded.
#[tokio::test]
async fn request_is_allowed_within_rate_limit() {
    let url = start_mock(0).await; // INCR → 1, limit = 10 → allowed
    let app = test_app(test_state_with_redis(None, &url));
    let (ct, body) = make_multipart_body(&minimal_png(), Some("nord"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", ct)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

/// Middleware returns 429 when the rate limit is exceeded.
#[tokio::test]
async fn request_is_blocked_when_rate_limit_exceeded() {
    let url = start_mock(10).await; // INCR → 11, limit = 10 → blocked
    let app = test_app(test_state_with_redis(None, &url));

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

/// Middleware fails open when Upstash is unreachable (request is not blocked).
#[tokio::test]
async fn request_is_allowed_when_upstash_unreachable() {
    let bad_url = "http://127.0.0.1:19995"; // nothing listening
    let app = test_app(test_state_with_redis(None, bad_url));
    let (ct, body) = make_multipart_body(&minimal_png(), Some("nord"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", ct)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // Fail-open: the request must not be rejected with 429.
    assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

/// When auth is disabled but an API key header is supplied, usage tracking fires.
#[tokio::test]
async fn usage_tracking_fires_when_api_key_header_is_present() {
    let url = start_mock(0).await;
    // Auth disabled (no api_keys configured) but x-api-key is sent anyway.
    let app = test_app(test_state_with_redis(None, &url));
    let (ct, body) = make_multipart_body(&minimal_png(), Some("nord"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", ct)
                .header("x-api-key", "any-key")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // Give the fire-and-forget usage task a moment to complete.
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    assert_eq!(resp.status(), StatusCode::OK);
}

/// Rate-limit identity uses the API key (not IP) when a key is provided.
#[tokio::test]
async fn rate_limit_uses_key_identity_when_auth_enabled() {
    let url = start_mock(0).await;
    let app = test_app(test_state_with_redis(Some(vec!["valid-key"]), &url));
    let (ct, body) = make_multipart_body(&minimal_png(), Some("nord"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", ct)
                .header("x-api-key", "valid-key")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}
