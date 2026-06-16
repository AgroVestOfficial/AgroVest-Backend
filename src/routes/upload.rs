use axum::{
    extract::{Multipart, State},
    routing::{get, post},
    Json, Router,
};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;

pub fn routes() -> Router<AppState> {
    Router::new().route("/upload", post(upload_file))
}

async fn upload_file(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut file_name = String::new();
    let mut mime_type = String::from("application/octet-stream");
    let mut file_bytes = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?
    {
        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("upload").to_string();
            if let Some(mime) = field.content_type() {
                mime_type = mime.to_string();
            }
            file_bytes = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?
                .to_vec();
        }
    }

    if file_bytes.is_empty() {
        return Err(ApiError::BadRequest("No file provided".into()));
    }

    let file_size = file_bytes.len() as i64;
    let result = state
        .ipfs
        .pin_file(&file_name, file_bytes, &mime_type)
        .await?;

    // Store metadata
    sqlx::query(
        r#"
        INSERT INTO ipfs_metadata (cid, original_name, mime_type, size_bytes, uploader)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (cid) DO NOTHING
        "#,
    )
    .bind(&result.cid)
    .bind(&file_name)
    .bind(&mime_type)
    .bind(file_size)
    .bind(&auth.address)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Database(e))?;

    Ok(Json(serde_json::json!({
        "cid": result.cid,
        "url": result.url,
    })))
}
