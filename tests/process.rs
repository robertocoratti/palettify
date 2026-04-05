// Integration tests for POST /api/v1/process

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;

use common::{make_multipart_body, make_multipart_full, minimal_png, test_app, test_state};

#[tokio::test]
async fn process_with_valid_palette_name_returns_200_png() {
    let app = test_app(test_state(None));
    let (content_type, body) = make_multipart_body(&minimal_png(), Some("nord"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("image/png"), "expected image/png, got {ct}");
}

#[tokio::test]
async fn process_with_unknown_palette_returns_404() {
    let app = test_app(test_state(None));
    let (content_type, body) =
        make_multipart_body(&minimal_png(), Some("does-not-exist"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn process_without_palette_returns_400() {
    let app = test_app(test_state(None));
    let (content_type, body) = make_multipart_body(&minimal_png(), None, None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn process_with_jpg_format_returns_jpeg_content_type() {
    let app = test_app(test_state(None));
    let (content_type, body) =
        make_multipart_body(&minimal_png(), Some("nord"), Some("jpg"));

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("image/jpeg"), "expected image/jpeg, got {ct}");
}

#[tokio::test]
async fn process_with_correct_api_key_returns_200() {
    let app = test_app(test_state(Some(vec!["test-key-123"])));
    let (content_type, body) = make_multipart_body(&minimal_png(), Some("nord"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .header("x-api-key", "test-key-123")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// ── algorithm field ─────────────────────────────────────────────────────────

#[tokio::test]
async fn process_with_floyd_steinberg_algorithm_returns_200() {
    let app = test_app(test_state(None));
    let (ct, body) =
        make_multipart_full(&minimal_png(), Some("nord"), None, None, Some("floyd-steinberg"));
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

#[tokio::test]
async fn process_with_ordered_algorithm_returns_200() {
    let app = test_app(test_state(None));
    let (ct, body) =
        make_multipart_full(&minimal_png(), Some("nord"), None, None, Some("ordered"));
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

#[tokio::test]
async fn process_with_unknown_algorithm_returns_400() {
    let app = test_app(test_state(None));
    let (ct, body) =
        make_multipart_full(&minimal_png(), Some("nord"), None, None, Some("invalid-algo"));
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
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── custom palette field ────────────────────────────────────────────────────

#[tokio::test]
async fn process_with_custom_palette_returns_200() {
    let app = test_app(test_state(None));
    let (ct, body) = make_multipart_full(
        &minimal_png(),
        None,
        Some("#ff0000\n#00ff00\n#0000ff"),
        None,
        None,
    );
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

// ── WebP output ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn process_with_webp_format_returns_webp_content_type() {
    let app = test_app(test_state(None));
    let (ct, body) =
        make_multipart_full(&minimal_png(), Some("nord"), None, Some("webp"), None);
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
    let content_type = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(content_type.contains("image/webp"), "expected image/webp, got {content_type}");
}

// ── invalid image bytes ──────────────────────────────────────────────────────

#[tokio::test]
async fn process_with_invalid_image_bytes_returns_400() {
    let app = test_app(test_state(None));
    let (ct, body) = make_multipart_full(
        b"this is not an image",
        Some("nord"),
        None,
        None,
        None,
    );
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
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── unknown multipart field (wildcard arm) ───────────────────────────────────

#[tokio::test]
async fn process_with_unknown_multipart_field_is_ignored() {
    // An unrecognised field name must be silently discarded (the _ => {} arm).
    let boundary = "boundary-unknown-field";
    let png = minimal_png();
    let mut body = Vec::new();
    let push = |b: &mut Vec<u8>, s: &str| b.extend_from_slice(s.as_bytes());

    push(&mut body, &format!("--{boundary}\r\n"));
    push(&mut body, "Content-Disposition: form-data; name=\"image\"; filename=\"t.png\"\r\n");
    push(&mut body, "Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(&png);
    push(&mut body, "\r\n");

    push(&mut body, &format!("--{boundary}\r\n"));
    push(&mut body, "Content-Disposition: form-data; name=\"totally_unknown_field\"\r\n\r\n");
    push(&mut body, "some value");
    push(&mut body, "\r\n");

    push(&mut body, &format!("--{boundary}\r\n"));
    push(&mut body, "Content-Disposition: form-data; name=\"palette_name\"\r\n\r\n");
    push(&mut body, "nord");
    push(&mut body, "\r\n");

    push(&mut body, &format!("--{boundary}--\r\n"));

    let app = test_app(test_state(None));
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// ── content-disposition ──────────────────────────────────────────────────────

#[tokio::test]
async fn process_response_includes_content_disposition() {
    let app = test_app(test_state(None));
    let (content_type, body) = make_multipart_body(&minimal_png(), Some("nord"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert!(
        resp.headers().contains_key("content-disposition"),
        "response should include Content-Disposition header",
    );
}
