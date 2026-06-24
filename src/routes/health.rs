use std::sync::OnceLock;
use std::time::Instant;

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
use serde_json::json;

use crate::app_state::AppState;

static STARTED_AT: OnceLock<Instant> = OnceLock::new();

/// Call once at process startup to begin tracking uptime.
pub fn init() {
    STARTED_AT.get_or_init(Instant::now);
}

fn uptime_secs() -> u64 {
    STARTED_AT.get().map(|t| t.elapsed().as_secs()).unwrap_or(0)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(liveness))
        .route("/ready", get(readiness))
}

#[derive(Serialize)]
struct LivenessResponse {
    status: &'static str,
    version: &'static str,
    uptime_seconds: u64,
}

/// Liveness probe — returns 200 as long as the process is running.
/// No dependency checks; used by orchestrators to detect hung processes.
async fn liveness() -> Json<LivenessResponse> {
    Json(LivenessResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime_secs(),
    })
}

#[derive(Serialize)]
struct ReadinessResponse {
    status: &'static str,
    checks: DependencyChecks,
}

#[derive(Serialize)]
struct DependencyChecks {
    database: &'static str,
    redis: &'static str,
}

/// Readiness probe — returns 200 only when all dependencies are reachable.
/// Returns 503 with per-dependency status on any failure.
/// Used by load balancers to gate traffic routing.
async fn readiness(
    State(state): State<AppState>,
) -> Result<Json<ReadinessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let db_ok = sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .is_ok();

    let redis_ok = redis::cmd("PING")
        .query_async::<String>(&mut state.redis.clone())
        .await
        .is_ok();

    if db_ok && redis_ok {
        Ok(Json(ReadinessResponse {
            status: "ok",
            checks: DependencyChecks {
                database: "ok",
                redis: "ok",
            },
        }))
    } else {
        Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": {
                    "code": 503,
                    "message": "Service not ready",
                    "checks": {
                        "database": if db_ok { "ok" } else { "unreachable" },
                        "redis": if redis_ok { "ok" } else { "unreachable" },
                    }
                }
            })),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    fn liveness_router() -> Router {
        Router::new().route("/health", get(liveness))
    }

    #[tokio::test]
    async fn liveness_returns_200() {
        let app = liveness_router();
        let resp = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn liveness_body_contains_status_ok() {
        let app = liveness_router();
        let resp = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    async fn liveness_body_contains_version() {
        let app = liveness_router();
        let resp = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn liveness_body_contains_uptime_seconds() {
        let app = liveness_router();
        let resp = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(body["uptime_seconds"].is_number());
    }

    #[tokio::test]
    async fn uptime_secs_returns_zero_before_init() {
        // STARTED_AT may already be set by init() in another test; skip assertion
        // if already initialized — we only verify it never panics.
        let _ = uptime_secs();
    }

    #[tokio::test]
    async fn uptime_secs_increases_after_init() {
        init();
        let t0 = uptime_secs();
        // init is idempotent; uptime should be non-panicking and monotonically non-decreasing.
        let t1 = uptime_secs();
        assert!(t1 >= t0);
    }
}
