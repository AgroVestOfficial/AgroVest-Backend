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
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn response_status_and_body(err: ApiError) -> (StatusCode, serde_json::Value) {
        let response = err.into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        (status, body)
    }

    #[tokio::test]
    async fn not_found_returns_404() {
        let (status, body) = response_status_and_body(ApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"]["code"], 404);
    }

    #[tokio::test]
    async fn unauthorized_returns_401() {
        let (status, body) = response_status_and_body(ApiError::Unauthorized).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"]["code"], 401);
    }

    #[tokio::test]
    async fn forbidden_returns_403() {
        let (status, body) = response_status_and_body(ApiError::Forbidden).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"]["code"], 403);
    }

    #[tokio::test]
    async fn bad_request_returns_400_with_message() {
        let (status, body) =
            response_status_and_body(ApiError::BadRequest("invalid input".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], 400);
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("invalid input"));
    }

    #[tokio::test]
    async fn conflict_returns_409() {
        let (status, body) = response_status_and_body(ApiError::Conflict("duplicate".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"]["code"], 409);
    }

    #[tokio::test]
    async fn internal_returns_500_with_generic_message() {
        let (status, body) =
            response_status_and_body(ApiError::Internal(anyhow::anyhow!("secret details"))).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"]["code"], 500);
        assert_eq!(body["error"]["message"], "Internal server error");
    }

    #[tokio::test]
    async fn blockchain_returns_502() {
        let (status, body) =
            response_status_and_body(ApiError::Blockchain("rpc timeout".into())).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(body["error"]["code"], 502);
    }

    #[tokio::test]
    async fn error_body_has_consistent_shape() {
        let (_, body) = response_status_and_body(ApiError::NotFound).await;
        assert!(body.get("error").is_some());
        assert!(body["error"].get("code").is_some());
        assert!(body["error"].get("message").is_some());
    }
}
