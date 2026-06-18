use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::models::challenge::CreateChallenge;
use crate::models::dispute::CreateDispute;
use crate::models::proposal::{CreateProposal, VoteRequest};
use crate::services::dao_service;
use crate::utils::pagination::PaginationParams;
use crate::utils::validators::ValidJson;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/proposals", get(list_proposals).post(create_proposal))
        .route("/proposals/{id}", get(get_proposal))
        .route("/proposals/{id}/vote", post(vote_proposal))
        .route("/challenges", get(list_challenges).post(create_challenge))
        .route("/disputes", get(list_disputes).post(create_dispute))
}

async fn list_proposals(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = dao_service::list_proposals(&state.db, &pagination).await?;
    Ok(Json(
        serde_json::to_value(result).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn get_proposal(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let proposal = dao_service::get_proposal(&state.db, id).await?;
    Ok(Json(
        serde_json::to_value(proposal).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn create_proposal(
    State(state): State<AppState>,
    auth: AuthUser,
    ValidJson(data): ValidJson<CreateProposal>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let proposal = dao_service::create_proposal(&state.db, &auth.address, data).await?;
    Ok(Json(
        serde_json::to_value(proposal).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn vote_proposal(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthUser,
    ValidJson(data): ValidJson<VoteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let vote = dao_service::vote_on_proposal(&state.db, &auth.address, id, &data.vote).await?;
    Ok(Json(
        serde_json::to_value(vote).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn list_challenges(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let challenges = dao_service::list_challenges(&state.db).await?;
    Ok(Json(
        serde_json::to_value(challenges).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn create_challenge(
    State(state): State<AppState>,
    auth: AuthUser,
    ValidJson(data): ValidJson<CreateChallenge>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let challenge = dao_service::create_challenge(&state.db, &auth.address, data).await?;
    Ok(Json(
        serde_json::to_value(challenge).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn list_disputes(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let disputes = dao_service::list_disputes(&state.db).await?;
    Ok(Json(
        serde_json::to_value(disputes).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn create_dispute(
    State(state): State<AppState>,
    auth: AuthUser,
    ValidJson(data): ValidJson<CreateDispute>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let dispute = dao_service::create_dispute(&state.db, &auth.address, data).await?;
    Ok(Json(
        serde_json::to_value(dispute).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

#[cfg(test)]
mod tests {
    //! Integration tests for `POST /api/v1/disputes`.
    //!
    //! These drive the real router (auth extractor + handler + service + SQL) and
    //! require a live Postgres and Redis, per AGENTS.md. Connection strings are
    //! taken from `DATABASE_URL` / `REDIS_URL`, defaulting to a local
    //! `agrovest_test` database and `redis://127.0.0.1:6379`.
    use crate::app_state::AppState;
    use crate::config::AppConfig;
    use crate::middleware::auth::Claims;
    use crate::routes::build_router;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;
    use sqlx::PgPool;
    use tower::ServiceExt;

    const JWT_SECRET: &str = "test-secret-for-dispute-integration-tests";

    fn env_or(key: &str, default: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| default.to_string())
    }

    fn test_config() -> AppConfig {
        AppConfig {
            database_url: env_or(
                "DATABASE_URL",
                "postgres://MAC@localhost:5432/agrovest_test",
            ),
            redis_url: env_or("REDIS_URL", "redis://127.0.0.1:6379"),
            jwt_secret: JWT_SECRET.to_string(),
            jwt_expiration_hours: 24,
            pinata_api_key: String::new(),
            pinata_secret_key: String::new(),
            ipfs_backend: "pinata".to_string(),
            ipfs_gateway_url: "https://gateway.pinata.cloud/ipfs".to_string(),
            soroban_rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            farm_contract_address: String::new(),
            investment_contract_address: String::new(),
            escrow_contract_address: String::new(),
            dao_contract_address: String::new(),
            indexer_poll_interval_secs: 5,
            server_host: "0.0.0.0".to_string(),
            server_port: 8080,
            cors_origins: vec!["http://localhost:3000".to_string()],
            rate_limit_global: 100,
            rate_limit_auth: 10,
            rate_limit_write: 30,
            trusted_proxies: vec![],
        }
    }

    async fn setup() -> AppState {
        let state = AppState::new(test_config())
            .await
            .expect("connect to test Postgres/Redis (see AGENTS.md)");
        sqlx::migrate!("./migrations")
            .run(&state.db)
            .await
            .expect("run migrations on the test database");
        state
    }

    /// A Stellar-length (56-char) address that is unique per `tag`, so parallel
    /// tests never collide on the `users` primary key or the dispute uniqueness rule.
    fn addr(tag: &str) -> String {
        format!("G{tag:0<55}").chars().take(56).collect()
    }

    async fn insert_user(pool: &PgPool, address: &str) {
        sqlx::query("INSERT INTO users (address) VALUES ($1) ON CONFLICT (address) DO NOTHING")
            .bind(address)
            .execute(pool)
            .await
            .expect("insert user");
    }

    /// Seeds a proposal and a challenge against it, returning the challenge id.
    async fn seed_challenge(
        pool: &PgPool,
        proposer: &str,
        challenger: &str,
        resolved: bool,
    ) -> i32 {
        insert_user(pool, proposer).await;
        insert_user(pool, challenger).await;

        let proposal_id: i32 = sqlx::query_scalar(
            r#"INSERT INTO proposals (title, created_at_onchain, ends_at, required_votes, proposer)
               VALUES ($1, $2, $3, $4, $5) RETURNING id"#,
        )
        .bind("Test proposal")
        .bind(0_i64)
        .bind(0_i64)
        .bind(1_i64)
        .bind(proposer)
        .fetch_one(pool)
        .await
        .expect("insert proposal");

        sqlx::query_scalar(
            r#"INSERT INTO challenges (proposal_id, description, challenger, resolved)
               VALUES ($1, $2, $3, $4) RETURNING id"#,
        )
        .bind(proposal_id)
        .bind(Some("test challenge"))
        .bind(challenger)
        .bind(resolved)
        .fetch_one(pool)
        .await
        .expect("insert challenge")
    }

    fn token_for(address: &str) -> String {
        let exp = (chrono::Utc::now().timestamp() + 3600) as usize;
        let claims = Claims {
            sub: address.to_string(),
            exp,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
        )
        .expect("encode test JWT")
    }

    fn post_dispute(token: Option<&str>, challenge_id: i32) -> Request<Body> {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/v1/disputes")
            .header("content-type", "application/json");
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        builder
            .body(Body::from(
                json!({ "challenge_id": challenge_id }).to_string(),
            ))
            .expect("build request")
    }

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read response body");
        serde_json::from_slice(&bytes).expect("parse response body as JSON")
    }

    // 1. No Authorization header -> 401 Unauthorized.
    #[tokio::test]
    async fn create_dispute_without_auth_is_unauthorized() {
        let state = setup().await;
        let challenge_id =
            seed_challenge(&state.db, &addr("noauth-prop"), &addr("noauth-chal"), false).await;

        let resp = build_router(state)
            .oneshot(post_dispute(None, challenge_id))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // 2. Valid JWT, but the user is neither challenger nor proposer -> 403 Forbidden.
    #[tokio::test]
    async fn create_dispute_by_non_party_is_forbidden() {
        let state = setup().await;
        let challenge_id =
            seed_challenge(&state.db, &addr("fbd-prop"), &addr("fbd-chal"), false).await;
        let token = token_for(&addr("fbd-outsider"));

        let resp = build_router(state)
            .oneshot(post_dispute(Some(&token), challenge_id))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // 3. Challenge id does not exist -> 404 Not Found.
    #[tokio::test]
    async fn create_dispute_for_missing_challenge_is_not_found() {
        let state = setup().await;
        let token = token_for(&addr("nf-user"));

        let resp = build_router(state)
            .oneshot(post_dispute(Some(&token), 2_000_000_000))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // 4. Challenge is already resolved -> 400 Bad Request.
    #[tokio::test]
    async fn create_dispute_on_resolved_challenge_is_bad_request() {
        let state = setup().await;
        let challenger = addr("res-chal");
        let challenge_id = seed_challenge(&state.db, &addr("res-prop"), &challenger, true).await;
        let token = token_for(&challenger);

        let resp = build_router(state)
            .oneshot(post_dispute(Some(&token), challenge_id))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // 5. A dispute already exists for the challenge -> 409 Conflict.
    #[tokio::test]
    async fn create_duplicate_dispute_is_conflict() {
        let state = setup().await;
        let challenger = addr("dup-chal");
        let challenge_id = seed_challenge(&state.db, &addr("dup-prop"), &challenger, false).await;
        let token = token_for(&challenger);
        let app = build_router(state);

        let first = app
            .clone()
            .oneshot(post_dispute(Some(&token), challenge_id))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);

        let second = app
            .oneshot(post_dispute(Some(&token), challenge_id))
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::CONFLICT);
    }

    // 6. Challenger raises a valid dispute -> 200 OK, arbitrator == auth.address.
    #[tokio::test]
    async fn challenger_creates_dispute_ok() {
        let state = setup().await;
        let challenger = addr("ok-chal-c");
        let challenge_id = seed_challenge(&state.db, &addr("ok-chal-p"), &challenger, false).await;
        let token = token_for(&challenger);

        let resp = build_router(state)
            .oneshot(post_dispute(Some(&token), challenge_id))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["arbitrator"], challenger);
        assert_eq!(body["challenge_id"], challenge_id);
    }

    // 7. The proposal's proposer raises a valid dispute -> 200 OK, arbitrator == auth.address.
    #[tokio::test]
    async fn proposer_creates_dispute_ok() {
        let state = setup().await;
        let proposer = addr("ok-prop-p");
        let challenge_id = seed_challenge(&state.db, &proposer, &addr("ok-prop-c"), false).await;
        let token = token_for(&proposer);

        let resp = build_router(state)
            .oneshot(post_dispute(Some(&token), challenge_id))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["arbitrator"], proposer);
        assert_eq!(body["challenge_id"], challenge_id);
    }
}
