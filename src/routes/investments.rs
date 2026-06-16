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
    let result =
        investment_service::list_investments(&state.db, &pagination, q.active.unwrap_or(false))
            .await?;
    Ok(Json(serde_json::to_value(result).unwrap()))
}

async fn get_investment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let investment = investment_service::get_investment(&state.db, id).await?;
    Ok(Json(serde_json::to_value(investment).unwrap()))
}

async fn get_investors(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let investors = investment_service::get_investors(&state.db, id).await?;
    Ok(Json(serde_json::to_value(investors).unwrap()))
}

async fn create_investment(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateInvestment>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let investment = investment_service::create_investment(&state.db, &auth.address, data).await?;
    Ok(Json(serde_json::to_value(investment).unwrap()))
}

async fn invest(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthUser,
    Json(data): Json<CreateInvestmentEntry>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let investor = investment_service::invest(&state.db, &auth.address, id, data.amount).await?;
    Ok(Json(serde_json::to_value(investor).unwrap()))
}
