// Integration tests for GET /api/v1/escrows/{id} authorization (issue #6).
// Each test drives the full Axum router via tower::ServiceExt::oneshot so the
// middleware stack — including AuthUser extraction — is exercised end-to-end.

mod helpers;

use axum::http::{Request, StatusCode};
use tower::ServiceExt;

/// Shared Stellar-format test addresses.
const BUYER: &str = "GAHJJJKMOKYE4RVPZEWZTKH5FVI4PA3VL7GK2LFNUBSGBV7REEX6XCLD";
const FARMER: &str = "GBVNQNSK5A7MYKNUVQNTXFPJBCRDM5RLKQE3QSEPAU3R3KXBYNRPVKM";
const STRANGER: &str = "GDQJUTQYK2MQX2VGDR2FYWLIYAQIEGXTQVTFEMGH532UDY6LPTN4TLI";
const JWT_SECRET: &str = "test-secret-key";

// ---------------------------------------------------------------------------
// Test 1 — unauthenticated request returns 401
// ---------------------------------------------------------------------------
#[tokio::test]
async fn unauthenticated_get_escrow_returns_401() {
    let app = test_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/escrows/1")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Helpers shared by tests 2-5 (require a live DB — marked #[ignore] for CI
// without a Postgres instance; run with `cargo test -- --include-ignored`).
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Test 2 — non-party authenticated request returns 404
// ---------------------------------------------------------------------------
#[tokio::test]
#[ignore = "requires live Postgres test database"]
async fn non_party_get_escrow_returns_404() {
    let app = test_app().await;
    let pool = test_pool().await;
    let fixture = helpers::escrow_fixture::seed_escrow(&pool, BUYER, FARMER).await;
    let token = helpers::jwt::mint_token(STRANGER, JWT_SECRET);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/escrows/{}", fixture.id))
                .header("authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Test 3 — buyer gets 200 with full escrow JSON
// ---------------------------------------------------------------------------
#[tokio::test]
#[ignore = "requires live Postgres test database"]
async fn buyer_get_escrow_returns_200() {
    let app = test_app().await;
    let pool = test_pool().await;
    let fixture = helpers::escrow_fixture::seed_escrow(&pool, BUYER, FARMER).await;
    let token = helpers::jwt::mint_token(BUYER, JWT_SECRET);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/escrows/{}", fixture.id))
                .header("authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["buyer"], BUYER);
    assert_eq!(json["farmer"], FARMER);
    assert_eq!(json["amount"], fixture.amount);
}

// ---------------------------------------------------------------------------
// Test 4 — farmer gets 200 with full escrow JSON
// ---------------------------------------------------------------------------
#[tokio::test]
#[ignore = "requires live Postgres test database"]
async fn farmer_get_escrow_returns_200() {
    let app = test_app().await;
    let pool = test_pool().await;
    let fixture = helpers::escrow_fixture::seed_escrow(&pool, BUYER, FARMER).await;
    let token = helpers::jwt::mint_token(FARMER, JWT_SECRET);

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/escrows/{}", fixture.id))
                .header("authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["buyer"], BUYER);
    assert_eq!(json["farmer"], FARMER);
}

// ---------------------------------------------------------------------------
// Test 5 — nonexistent escrow returns 404 regardless of auth
// ---------------------------------------------------------------------------
#[tokio::test]
#[ignore = "requires live Postgres test database"]
async fn nonexistent_escrow_returns_404() {
    let app = test_app().await;
    let token = helpers::jwt::mint_token(BUYER, JWT_SECRET);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/escrows/99999")
                .header("authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Test 6 — malformed / tampered JWT returns 401, not 500
// ---------------------------------------------------------------------------
#[tokio::test]
async fn tampered_jwt_get_escrow_returns_401() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/escrows/1")
                .header("authorization", "Bearer this.is.not.a.valid.jwt")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

async fn test_pool() -> sqlx::PgPool {
    use sqlx::postgres::PgPoolOptions;
    use std::env;
    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/agrovest_test".to_owned());
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await
        .expect("test DB connection failed")
}

async fn test_app() -> axum::Router {
    // Build a real AppState using the factory so all fields (redis, http_client,
    // ipfs) are correctly initialised. AppConfig::from_env() requires only
    // DATABASE_URL and JWT_SECRET — all other fields have safe defaults.
    // CI provides both via the postgres + redis service containers.
    use agrovest_backend::app_state::AppState;
    use agrovest_backend::config::AppConfig;
    use agrovest_backend::routes::build_router;

    let config =
        AppConfig::from_env().expect("test config: DATABASE_URL and JWT_SECRET must be set");
    let state = AppState::new(config)
        .await
        .expect("test AppState init failed (check DATABASE_URL and REDIS_URL)");
    build_router(state)
}
