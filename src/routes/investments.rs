use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::models::investment::CreateInvestment;
use crate::models::investor::CreateInvestmentEntry;
use crate::services::investment_service;
use crate::utils::pagination::PaginationParams;
use crate::utils::validators::ValidJson;

#[derive(serde::Deserialize)]
pub struct InvestmentQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub active: Option<bool>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/investments",
            get(list_investments).post(create_investment),
        )
        .route("/investments/{id}", get(get_investment))
        .route("/investments/{id}/investors", get(get_investors))
        .route("/investments/{id}/invest", post(invest))
}

async fn list_investments(
    State(state): State<AppState>,
    Query(q): Query<InvestmentQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pagination = PaginationParams {
        page: q.page,
        per_page: q.per_page,
    };
    let mut redis = state.redis.clone();
    let result = investment_service::list_investments(
        &state.db,
        &mut redis,
        &pagination,
        q.active.unwrap_or(false),
    )
    .await?;
    Ok(Json(
        serde_json::to_value(result).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn get_investment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut redis = state.redis.clone();
    let investment = investment_service::get_investment(&state.db, &mut redis, id).await?;
    Ok(Json(
        serde_json::to_value(investment).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn get_investors(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let investors = investment_service::get_investors(&state.db, id).await?;
    Ok(Json(
        serde_json::to_value(investors).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn create_investment(
    State(state): State<AppState>,
    auth: AuthUser,
    ValidJson(data): ValidJson<CreateInvestment>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut redis = state.redis.clone();
    let investment =
        investment_service::create_investment(&state.db, &mut redis, &auth.address, data).await?;
    Ok(Json(
        serde_json::to_value(investment).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn invest(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthUser,
    ValidJson(data): ValidJson<CreateInvestmentEntry>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut redis = state.redis.clone();
    let investor =
        investment_service::invest(&state.db, &mut redis, &auth.address, id, data.amount).await?;
    Ok(Json(
        serde_json::to_value(investor).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}
