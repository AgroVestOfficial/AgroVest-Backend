pub mod auth;
pub mod cart;
pub mod dao;
pub mod escrows;
pub mod farms;
pub mod indexer;
pub mod investments;
pub mod products;
pub mod reviews;
pub mod upload;
pub mod users;

use crate::app_state::AppState;
use axum::http::HeaderValue;
use axum::Router;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;

fn build_allowed_origins(origins: &[String]) -> Vec<HeaderValue> {
    origins
        .iter()
        .filter_map(|o| o.parse::<HeaderValue>().ok())
        .collect()
}

pub fn build_router(state: AppState) -> Router {
    let allowed_origins = build_allowed_origins(&state.config.cors_origins);

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        .merge(auth::routes())
        .merge(users::routes())
        .merge(farms::routes())
        .merge(products::routes())
        .merge(reviews::routes())
        .merge(cart::routes())
        .merge(investments::routes())
        .merge(escrows::routes())
        .merge(dao::routes())
        .merge(upload::routes())
        .merge(indexer::routes());

    Router::new()
        .nest("/api/v1", api)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_http_and_https_origins_parse_correctly() {
        let origins = vec![
            "http://localhost:3000".to_string(),
            "https://app.agrovest.io".to_string(),
        ];
        let values = build_allowed_origins(&origins);
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], "http://localhost:3000");
        assert_eq!(values[1], "https://app.agrovest.io");
    }

    #[test]
    fn empty_origins_list_yields_empty_result_denying_all_cross_origin_requests() {
        let origins: Vec<String> = vec![];
        let values = build_allowed_origins(&origins);
        assert!(values.is_empty());
    }

    #[test]
    fn origin_with_embedded_newline_is_filtered_out() {
        let origins = vec![
            "http://evil.com\nX-Injected: header".to_string(),
            "https://app.agrovest.io".to_string(),
        ];
        let values = build_allowed_origins(&origins);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], "https://app.agrovest.io");
    }

    #[test]
    fn origin_with_carriage_return_is_filtered_out() {
        let origins = vec![
            "http://evil.com\rX-Injected: header".to_string(),
            "http://localhost:3000".to_string(),
        ];
        let values = build_allowed_origins(&origins);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], "http://localhost:3000");
    }

    #[test]
    fn origin_with_null_byte_is_filtered_out() {
        let origins = vec![
            "http://evil\x00.com".to_string(),
            "https://staging.agrovest.io".to_string(),
        ];
        let values = build_allowed_origins(&origins);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], "https://staging.agrovest.io");
    }

    #[test]
    fn all_invalid_origins_filtered_leaves_empty_deny_all_list() {
        let origins = vec![
            "http://evil.com\n".to_string(),
            "http://other.com\r".to_string(),
            "http://null\x00byte.com".to_string(),
        ];
        let values = build_allowed_origins(&origins);
        assert!(values.is_empty());
    }

    #[test]
    fn multiple_valid_origins_including_ports_are_all_accepted() {
        let origins = vec![
            "http://localhost:3000".to_string(),
            "http://localhost:5173".to_string(),
            "https://staging.agrovest.io".to_string(),
            "https://app.agrovest.io".to_string(),
        ];
        let values = build_allowed_origins(&origins);
        assert_eq!(values.len(), 4);
    }

    #[test]
    fn config_parses_comma_separated_cors_origins_with_whitespace_trimming() {
        let raw = "http://localhost:3000, https://app.agrovest.io , https://staging.agrovest.io";
        let parsed: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).collect();

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], "http://localhost:3000");
        assert_eq!(parsed[1], "https://app.agrovest.io");
        assert_eq!(parsed[2], "https://staging.agrovest.io");

        let values = build_allowed_origins(&parsed);
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn single_default_origin_from_config_is_accepted() {
        let origins = vec!["http://localhost:3000".to_string()];
        let values = build_allowed_origins(&origins);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], "http://localhost:3000");
    }
}
