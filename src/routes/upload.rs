use axum::{
    extract::{Multipart, State},
    routing::post,
    Json, Router,
};

use crate::app_state::AppState;
use crate::error::ApiError;
use crate::middleware::auth::AuthUser;

// Security constants for file uploads
const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "application/pdf",
];

const ALLOWED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "pdf"];

const MAX_MULTIPART_PARTS: usize = 10;

pub fn routes() -> Router<AppState> {
    Router::new().route("/upload", post(upload_file))
}

async fn upload_file(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let max_file_size_bytes = state.config.max_upload_size_mb * 1024 * 1024;
    let mut file_name = String::new();
    let mut mime_type = String::from("application/octet-stream");
    let mut file_bytes = Vec::new();
    let mut part_count = 0;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {}", e)))?
    {
        // Limit number of multipart parts (defense against multipart bombs)
        part_count += 1;
        if part_count > MAX_MULTIPART_PARTS {
            return Err(ApiError::BadRequest(
                "Too many multipart parts: maximum 10 allowed".into(),
            ));
        }

        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("upload").to_string();
            if let Some(mime) = field.content_type() {
                mime_type = mime.to_string();
            }

            // Validate content type
            if !ALLOWED_MIME_TYPES.contains(&mime_type.as_str()) {
                return Err(ApiError::BadRequest(format!(
                    "File type not allowed: {}. Allowed types: {:?}",
                    mime_type, ALLOWED_MIME_TYPES
                )));
            }

            // Validate file extension
            let extension = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
            if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
                return Err(ApiError::BadRequest(format!(
                    "File extension not allowed: .{}. Allowed: {:?}",
                    extension, ALLOWED_EXTENSIONS
                )));
            }

            // Read file data and validate size (defense against large uploads)
            let file_data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Read error: {}", e)))?;

            if file_data.len() > max_file_size_bytes {
                return Err(ApiError::BadRequest(format!(
                    "File too large: maximum {}MB allowed",
                    state.config.max_upload_size_mb
                )));
            }

            file_bytes = file_data.to_vec();
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
    .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "cid": result.cid,
        "url": result.url,
    })))
}
