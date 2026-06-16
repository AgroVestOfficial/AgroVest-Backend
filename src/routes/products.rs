use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::models::product::{CreateProduct, ProductFilter, UpdateProduct};
use crate::services::product_service;
use crate::utils::pagination::PaginationParams;

#[derive(serde::Deserialize)]
pub struct ProductQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub category: Option<String>,
    pub farm_id: Option<i32>,
    pub sold: Option<bool>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/products", get(list_products).post(create_product))
        .route("/products/{id}", get(get_product).put(update_product))
        .route("/farms/{farm_id}/products", get(get_farm_products))
}

async fn list_products(
    State(state): State<AppState>,
    Query(q): Query<ProductQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pagination = PaginationParams {
        page: q.page,
        per_page: q.per_page,
    };
    let filter = ProductFilter {
        category: q.category,
        farm_id: q.farm_id,
        sold: q.sold,
    };
    let result = product_service::list_products(&state.db, &pagination, &filter).await?;
    Ok(Json(
        serde_json::to_value(result).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let product = product_service::get_product(&state.db, id).await?;
    Ok(Json(
        serde_json::to_value(product).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn get_farm_products(
    State(state): State<AppState>,
    Path(farm_id): Path<i32>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = product_service::get_farm_products(&state.db, farm_id, &pagination).await?;
    Ok(Json(
        serde_json::to_value(result).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn create_product(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateProduct>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let product = product_service::create_product(&state.db, &auth.address, data).await?;
    Ok(Json(
        serde_json::to_value(product).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthUser,
    Json(data): Json<UpdateProduct>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let product = product_service::update_product(&state.db, id, &auth.address, data).await?;
    Ok(Json(
        serde_json::to_value(product).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}
