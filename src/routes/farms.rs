use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::models::farm::{CreateFarm, UpdateFarm};
use crate::services::{farm_service, user_service};
use crate::utils::pagination::PaginationParams;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/farms", get(list_farms).post(create_farm))
        .route("/farms/{id}", get(get_farm).put(update_farm))
        .route("/farms/by-address/{address}", get(get_farm_by_address))
}

async fn list_farms(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = farm_service::list_farms(&state.db, &pagination).await?;
    Ok(Json(
        serde_json::to_value(result).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn get_farm(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let farm = farm_service::get_farm(&state.db, id).await?;
    Ok(Json(
        serde_json::to_value(farm).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn get_farm_by_address(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let farm = farm_service::get_farm_by_address(&state.db, &address).await?;
    Ok(Json(
        serde_json::to_value(farm).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn create_farm(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateFarm>,
) -> Result<Json<serde_json::Value>, ApiError> {
    user_service::upsert_user(&state.db, &auth.address).await?;
    let farm = farm_service::create_farm(&state.db, &auth.address, data).await?;
    Ok(Json(
        serde_json::to_value(farm).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn update_farm(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthUser,
    Json(data): Json<UpdateFarm>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let farm = farm_service::update_farm(&state.db, id, &auth.address, data).await?;
    Ok(Json(
        serde_json::to_value(farm).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}
