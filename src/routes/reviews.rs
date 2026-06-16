use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::models::review::CreateReview;
use crate::services::review_service;

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/products/{product_id}/reviews",
        get(get_reviews).post(create_review),
    )
}

async fn get_reviews(
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let reviews = review_service::get_product_reviews(&state.db, product_id).await?;
    Ok(Json(serde_json::to_value(reviews).unwrap()))
}

async fn create_review(
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
    auth: AuthUser,
    Json(data): Json<CreateReview>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let review = review_service::create_review(&state.db, &auth.address, product_id, data).await?;
    Ok(Json(serde_json::to_value(review).unwrap()))
}
