use axum::{
    extract::{Path, Query, State},
    routing::{get, put},
    Json, Router,
};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::models::escrow::{CreateEscrow, EscrowFilter, EscrowStatus};
use crate::services::escrow_service;
use crate::utils::pagination::PaginationParams;

#[derive(serde::Deserialize)]
pub struct EscrowQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub role: Option<String>,
    pub status: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/escrows", get(list_escrows).post(create_escrow))
        .route("/escrows/{id}", get(get_escrow))
        .route("/escrows/{id}/approve", put(approve_delivery))
        .route("/escrows/{id}/dispute", put(raise_dispute))
}

async fn list_escrows(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<EscrowQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pagination = PaginationParams {
        page: q.page,
        per_page: q.per_page,
    };
    let filter = EscrowFilter {
        role: q.role,
        status: q.status,
    };
    let result =
        escrow_service::list_escrows(&state.db, &auth.address, &pagination, &filter).await?;
    Ok(Json(
        serde_json::to_value(result).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn get_escrow(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let escrow = escrow_service::get_escrow(&state.db, id).await?;
    Ok(Json(
        serde_json::to_value(escrow).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn create_escrow(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateEscrow>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let escrow = escrow_service::create_escrow(&state.db, &auth.address, data).await?;
    Ok(Json(
        serde_json::to_value(escrow).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn approve_delivery(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let escrow = escrow_service::get_escrow(&state.db, id).await?;
    if escrow.buyer != auth.address {
        return Err(ApiError::Forbidden);
    }
    let updated =
        escrow_service::update_escrow_status(&state.db, id, EscrowStatus::Complete).await?;
    Ok(Json(
        serde_json::to_value(updated).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn raise_dispute(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let escrow = escrow_service::get_escrow(&state.db, id).await?;
    if escrow.buyer != auth.address && escrow.farmer != auth.address {
        return Err(ApiError::Forbidden);
    }
    let updated =
        escrow_service::update_escrow_status(&state.db, id, EscrowStatus::Dispute).await?;
    Ok(Json(
        serde_json::to_value(updated).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}
