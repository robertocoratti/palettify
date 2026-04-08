// Integration tests for POST /api/v1/process

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;

use common::{make_multipart_body, make_multipart_full, minimal_png, test_app, test_state, test_state_with_pixel_limit};

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

// ── multipart field read errors (non-UTF-8 content) ─────────────────────────

fn multipart_with_invalid_utf8_field(field_name: &str) -> (String, Vec<u8>) {
    let boundary = "boundary-utf8-error";
    let png = minimal_png();
    let mut body = Vec::new();
    let push = |b: &mut Vec<u8>, s: &str| b.extend_from_slice(s.as_bytes());

    // Always include a valid image field.
    push(&mut body, &format!("--{boundary}\r\n"));
    push(&mut body, "Content-Disposition: form-data; name=\"image\"; filename=\"t.png\"\r\n");
    push(&mut body, "Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(&png);
    push(&mut body, "\r\n");

    // Inject the target field with bytes that are not valid UTF-8.
    push(&mut body, &format!("--{boundary}\r\n"));
    push(&mut body, &format!("Content-Disposition: form-data; name=\"{field_name}\"\r\n\r\n"));
    body.extend_from_slice(&[0xFF, 0xFE, 0x00]); // invalid UTF-8
    push(&mut body, "\r\n");

    push(&mut body, &format!("--{boundary}--\r\n"));
    (format!("multipart/form-data; boundary={boundary}"), body)
}

#[tokio::test]
async fn process_with_garbage_palette_name_returns_client_error() {
    // axum multipart reads bytes lossily — the garbage palette name will not be
    // found in the palette map, resulting in a 404 client error.
    let app = test_app(test_state(None));
    let (ct, body) = multipart_with_invalid_utf8_field("palette_name");
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
    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn process_with_invalid_utf8_algorithm_returns_400() {
    let app = test_app(test_state(None));
    let (ct, body) = multipart_with_invalid_utf8_field("algorithm");
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
    // Without a palette the request may be rejected for a different reason (400);
    // the important thing is it does not panic and returns a client error.
    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn process_with_invalid_utf8_palette_field_returns_400() {
    let app = test_app(test_state(None));
    let (ct, body) = multipart_with_invalid_utf8_field("palette");
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
    assert!(resp.status().is_client_error());
}

// ── pixel limit ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn process_with_image_exceeding_pixel_limit_returns_400() {
    // minimal_png() is 1×1 = 1 pixel; set limit to 0 so it is exceeded.
    let app = test_app(test_state_with_pixel_limit(0));
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

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ── custom palette with empty lines (exercises the filter closure) ───────────

#[tokio::test]
async fn process_with_palette_containing_empty_lines_returns_200() {
    let app = test_app(test_state(None));
    // Palette with leading/trailing blank lines; filter(|l| !l.is_empty()) must strip them.
    let (ct, body) = make_multipart_full(
        &minimal_png(),
        None,
        Some("\n#ff0000\n\n#00ff00\n"),
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
