use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::services::{auth_service, user_service};

#[derive(Deserialize)]
pub struct NonceRequest {
    pub address: String,
}

#[derive(Serialize)]
pub struct NonceResponse {
    pub nonce: String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub address: String,
    pub signature: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub address: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/nonce", post(get_nonce))
        .route("/auth/verify", post(verify_signature))
        .route("/auth/me", get(me))
}

async fn get_nonce(
    State(mut state): State<AppState>,
    Json(req): Json<NonceRequest>,
) -> Result<Json<NonceResponse>, ApiError> {
    let nonce = auth_service::generate_nonce(&mut state.redis, &req.address).await?;
    Ok(Json(NonceResponse { nonce }))
}

async fn verify_signature(
    State(mut state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    let token = auth_service::verify_and_issue_jwt(
        &mut state.redis,
        &req.address,
        &req.signature,
        &state.config.jwt_secret,
        state.config.jwt_expiration_hours,
    )
    .await?;

    // Ensure user exists in DB
    user_service::upsert_user(&state.db, &req.address).await?;

    Ok(Json(TokenResponse { token }))
}

async fn me(user: AuthUser) -> Result<Json<MeResponse>, ApiError> {
    Ok(Json(MeResponse {
        address: user.address,
    }))
}
