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
    ValidJson(data): ValidJson<CreateDispute>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let dispute = dao_service::create_dispute(&state.db, data).await?;
    Ok(Json(
        serde_json::to_value(dispute).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}
