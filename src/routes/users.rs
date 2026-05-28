use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;
use crate::models::user::UpdateUser;
use crate::services::user_service;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users/{address}", get(get_user))
        .route("/users/{address}", put(update_user))
}

async fn get_user(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = user_service::get_user(&state.db, &address).await?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}

async fn update_user(
    State(state): State<AppState>,
    Path(address): Path<String>,
    auth: AuthUser,
    Json(update): Json<UpdateUser>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if auth.address != address {
        return Err(ApiError::Forbidden);
    }
    let user = user_service::update_user(&state.db, &address, update).await?;
    Ok(Json(serde_json::to_value(user).unwrap()))
}
