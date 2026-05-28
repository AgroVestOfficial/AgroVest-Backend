mod app_state;
mod blockchain;
mod config;
mod db;
mod error;
mod middleware;
mod models;
mod routes;
mod services;
mod utils;

use app_state::AppState;
use config::AppConfig;
use routes::build_router;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("agrovest_backend=info".parse()?))
        .init();

    let config = AppConfig::from_env()?;
    let state = AppState::new(config).await?;

    tracing::info!("Starting server on {}:{}", state.config.server_host, state.config.server_port);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", state.config.server_host, state.config.server_port)).await?;

    axum::serve(listener, build_router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}
