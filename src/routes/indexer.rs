use axum::{extract::State, routing::get, Json, Router};

use crate::app_state::AppState;
use crate::error::ApiError;

pub fn routes() -> Router<AppState> {
    Router::new().route("/indexer/status", get(indexer_status))
}

async fn indexer_status(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = sqlx::query_as::<_, (String, Option<i64>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT contract_address, synced_height, last_synced_at FROM indexer_state",
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let statuses: Vec<serde_json::Value> = rows
        .iter()
        .map(|(addr, height, synced)| {
            serde_json::json!({
                "contract_address": addr,
                "synced_height": height,
                "last_synced_at": synced,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "contracts": statuses })))
}
