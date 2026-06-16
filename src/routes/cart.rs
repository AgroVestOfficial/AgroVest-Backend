use axum::{
    extract::State,
    routing::{delete, get},
    Json, Router,
};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::models::cart_item::AddToCart;
use crate::services::cart_service;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cart", get(get_cart).post(add_to_cart).delete(clear_cart))
        .route("/cart/{product_id}", delete(remove_from_cart))
}

async fn get_cart(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let items = cart_service::get_cart(&state.db, &auth.address).await?;
    Ok(Json(
        serde_json::to_value(items).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn add_to_cart(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<AddToCart>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let item = cart_service::add_to_cart(&state.db, &auth.address, data.product_id).await?;
    Ok(Json(
        serde_json::to_value(item).map_err(|e| ApiError::Internal(e.into()))?,
    ))
}

async fn remove_from_cart(
    State(state): State<AppState>,
    axum::extract::Path(product_id): axum::extract::Path<i32>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    cart_service::remove_from_cart(&state.db, &auth.address, product_id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

async fn clear_cart(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    cart_service::clear_cart(&state.db, &auth.address).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}
