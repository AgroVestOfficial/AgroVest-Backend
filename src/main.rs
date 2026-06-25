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
use blockchain::soroban_client::SorobanClient;
use config::AppConfig;
use routes::build_router;
use services::indexer_service::{ContractConfig, IndexerService};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    routes::health::init();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("agrovest_backend=info".parse()?),
        )
        .init();

    let config = AppConfig::from_env()?;
    let mut state = AppState::new(config).await?;

    // Set up indexer if enabled
    let cancel_indexer = CancellationToken::new();
    if state.config.enable_indexer {
        let soroban = SorobanClient::new(
            state.config.soroban_rpc_url.clone(),
            state.http_client.clone(),
        );

        let contracts = vec![
            ContractConfig {
                address: state.config.farm_contract_address.clone(),
                domain: "farm".to_string(),
            },
            ContractConfig {
                address: state.config.investment_contract_address.clone(),
                domain: "investment".to_string(),
            },
            ContractConfig {
                address: state.config.escrow_contract_address.clone(),
                domain: "escrow".to_string(),
            },
            ContractConfig {
                address: state.config.dao_contract_address.clone(),
                domain: "dao".to_string(),
            },
        ];

        let indexer = IndexerService::new(
            soroban,
            state.db.clone(),
            state.config.indexer_poll_interval_secs,
            contracts,
        );
        let indexer = Arc::new(indexer);
        indexer.clone().start(cancel_indexer.clone());
        state.indexer = Some(indexer);

        tracing::info!("Blockchain indexer started");
    }

    tracing::info!(
        "Starting server on {}:{}",
        state.config.server_host,
        state.config.server_port
    );

    let listener = tokio::net::TcpListener::bind(format!(
        "{}:{}",
        state.config.server_host, state.config.server_port
    ))
    .await?;

    // ConnectInfo is required by the rate limit middleware to read the
    // direct TCP peer address before considering proxy headers.
    let app = build_router(state).into_make_service_with_connect_info::<SocketAddr>();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(cancel_indexer))
        .await?;

    Ok(())
}

async fn shutdown_signal(cancel_indexer: CancellationToken) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
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
    cancel_indexer.cancel();
}
