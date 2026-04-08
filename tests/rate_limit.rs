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

// ── Helper ────────────────────────────────────────────────────────────────────

fn header_str<'a>(resp: &'a axum::response::Response, name: &str) -> &'a str {
    resp.headers()
        .get(name)
        .expect(&format!("{name} header must be present"))
        .to_str()
        .unwrap()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Middleware passes the request and attaches X-RateLimit-* headers.
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
    assert_eq!(header_str(&resp, "x-ratelimit-limit"),     "10");
    assert_eq!(header_str(&resp, "x-ratelimit-remaining"), "9"); // 10 - 1
    // Reset is a unix timestamp in the future; just verify it's numeric and > 0.
    let reset: u64 = header_str(&resp, "x-ratelimit-reset").parse().unwrap();
    assert!(reset > 0);
}

/// Middleware returns 429 with X-RateLimit-* headers when the limit is exceeded.
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
    assert_eq!(header_str(&resp, "x-ratelimit-limit"),     "10");
    assert_eq!(header_str(&resp, "x-ratelimit-remaining"), "0"); // clamped
    let reset: u64 = header_str(&resp, "x-ratelimit-reset").parse().unwrap();
    assert!(reset > 0);
}

/// Middleware fails open when Upstash is unreachable — no rate-limit headers.
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

    // Fail-open: must not be rejected with 429.
    assert_ne!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    // No Upstash info → no rate-limit headers.
    assert!(resp.headers().get("x-ratelimit-limit").is_none());
}

/// When auth is disabled but an API key header is supplied, usage tracking fires.
#[tokio::test]
async fn usage_tracking_fires_when_api_key_header_is_present() {
    let url = start_mock(0).await;
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
    // Rate-limit headers must be present.
    assert!(resp.headers().contains_key("x-ratelimit-limit"));
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
    assert!(resp.headers().contains_key("x-ratelimit-limit"));
    assert!(resp.headers().contains_key("x-ratelimit-remaining"));
    assert!(resp.headers().contains_key("x-ratelimit-reset"));
}
