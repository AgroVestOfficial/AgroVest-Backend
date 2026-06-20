//! Integration tests for file upload security validations.
//! Tests the /api/v1/upload endpoint for size limits, content-type validation,
//! file extension validation, and multipart bomb protection.

mod helpers;

use axum::http::StatusCode;
use helpers::jwt;

const JWT_SECRET: &str = "test-secret-key";
const TEST_FARMER: &str = "GBVNQNSK5A7MYKNUVQNTXFPJBCRDM5RLKQE3QSEPAU3R3KXBYNRPVKM";

/// Test helper to build multipart form data
fn build_multipart(
    file_name: &str,
    content_type: &str,
    file_data: &[u8],
    extra_parts: usize,
) -> Vec<u8> {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let mut body = Vec::new();

    // Add extra empty parts to test multipart bomb limits
    for _ in 0..extra_parts {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"extra\"\r\n");
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(b"\r\n");
    }

    // Add actual file part
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
            file_name
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {}\r\n", content_type).as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(file_data);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    body
}

// Tests (marked #[ignore] because they require live database/IPFS)

/// Test 1: Valid file upload succeeds (sanity check)
#[tokio::test]
#[ignore] // Requires live database
async fn valid_jpg_upload_succeeds() {
    use agrovest_backend::app_state::AppState;
    use agrovest_backend::config::AppConfig;
    use agrovest_backend::routes::build_router;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let config = AppConfig::from_env().expect("DATABASE_URL and JWT_SECRET must be set");
    let state = AppState::new(config).await.expect("AppState init failed");
    let app = build_router(state);

    let token = jwt::mint_token(TEST_FARMER, JWT_SECRET);
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let multipart_data = build_multipart("test.jpg", "image/jpeg", &[0xFF; 100], 0);

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/upload")
        .header("authorization", format!("Bearer {}", token))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(multipart_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "valid upload should succeed"
    );
}

/// Test 2: Disallowed content type returns 400
#[tokio::test]
#[ignore] // Requires live database
async fn disallowed_content_type_returns_400() {
    use agrovest_backend::app_state::AppState;
    use agrovest_backend::config::AppConfig;
    use agrovest_backend::routes::build_router;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let config = AppConfig::from_env().expect("DATABASE_URL and JWT_SECRET must be set");
    let state = AppState::new(config).await.expect("AppState init failed");
    let app = build_router(state);

    let token = jwt::mint_token(TEST_FARMER, JWT_SECRET);
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    // Send as application/octet-stream (disallowed type)
    let multipart_data =
        build_multipart("malware.exe", "application/octet-stream", &[0x4D, 0x5A], 0);

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/upload")
        .header("authorization", format!("Bearer {}", token))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(multipart_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "disallowed content type should return 400"
    );
}

/// Test 3: Disallowed file extension returns 400
#[tokio::test]
#[ignore] // Requires live database
async fn disallowed_file_extension_returns_400() {
    use agrovest_backend::app_state::AppState;
    use agrovest_backend::config::AppConfig;
    use agrovest_backend::routes::build_router;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let config = AppConfig::from_env().expect("DATABASE_URL and JWT_SECRET must be set");
    let state = AppState::new(config).await.expect("AppState init failed");
    let app = build_router(state);

    let token = jwt::mint_token(TEST_FARMER, JWT_SECRET);
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let multipart_data = build_multipart("script.js", "text/javascript", b"console.log('xss');", 0);

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/upload")
        .header("authorization", format!("Bearer {}", token))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(multipart_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "disallowed file extension should return 400"
    );
}

/// Test 4: Multipart bomb (excessive parts) returns 400
#[tokio::test]
#[ignore] // Requires live database
async fn multipart_bomb_returns_400() {
    use agrovest_backend::app_state::AppState;
    use agrovest_backend::config::AppConfig;
    use agrovest_backend::routes::build_router;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let config = AppConfig::from_env().expect("DATABASE_URL and JWT_SECRET must be set");
    let state = AppState::new(config).await.expect("AppState init failed");
    let app = build_router(state);

    let token = jwt::mint_token(TEST_FARMER, JWT_SECRET);
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    // Request with 15 extra parts (plus 1 for file = 16 total, exceeds limit of 10)
    let multipart_data = build_multipart("test.jpg", "image/jpeg", &[0xFF; 100], 15);

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/upload")
        .header("authorization", format!("Bearer {}", token))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(multipart_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "multipart bomb (>10 parts) should return 400"
    );
}

/// Test 5: Upload exceeding size limit returns 400 from handler
/// (RequestBodyLimitLayer will return 413 before reaching handler for very large requests)
#[tokio::test]
#[ignore] // Requires live database and would need huge payload
async fn oversized_upload_returns_error() {
    use agrovest_backend::app_state::AppState;
    use agrovest_backend::config::AppConfig;
    use agrovest_backend::routes::build_router;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let config = AppConfig::from_env().expect("DATABASE_URL and JWT_SECRET must be set");
    let state = AppState::new(config).await.expect("AppState init failed");
    // Config has max_upload_size_mb = 10, so create 11MB file
    let oversized_data = vec![0u8; 11 * 1024 * 1024];
    let app = build_router(state);

    let token = jwt::mint_token(TEST_FARMER, JWT_SECRET);
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let multipart_data = build_multipart("huge.jpg", "image/jpeg", &oversized_data, 0);

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/upload")
        .header("authorization", format!("Bearer {}", token))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(multipart_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should be 413 from RequestBodyLimitLayer or 400 from handler
    assert!(
        response.status() == StatusCode::PAYLOAD_TOO_LARGE
            || response.status() == StatusCode::BAD_REQUEST,
        "oversized upload should return 413 or 400, got {}",
        response.status()
    );
}

/// Test 6: Unauthenticated upload returns 401
#[tokio::test]
#[ignore] // Requires live database
async fn unauthenticated_upload_returns_401() {
    use agrovest_backend::app_state::AppState;
    use agrovest_backend::config::AppConfig;
    use agrovest_backend::routes::build_router;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let config = AppConfig::from_env().expect("DATABASE_URL and JWT_SECRET must be set");
    let state = AppState::new(config).await.expect("AppState init failed");
    let app = build_router(state);

    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let multipart_data = build_multipart("test.jpg", "image/jpeg", &[0xFF; 100], 0);

    // No authorization header
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(multipart_data))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "unauthenticated upload should return 401"
    );
}
