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
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
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
