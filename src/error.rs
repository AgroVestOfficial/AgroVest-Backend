use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found")]
    NotFound,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Forbidden")]
    Forbidden,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Blockchain error: {0}")]
    Blockchain(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            ApiError::Internal(_) => {
                tracing::error!("Internal error: {:?}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            ApiError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            ApiError::Blockchain(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
        };

        let body = Json(json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}
