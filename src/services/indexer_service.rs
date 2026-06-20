#![allow(dead_code)]

use crate::blockchain::soroban_client::SorobanClient;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct IndexerService {
    soroban: SorobanClient,
    pool: PgPool,
    poll_interval: u64,
    contracts: Vec<ContractConfig>,
}

pub struct ContractConfig {
    pub address: String,
    pub domain: String,
}

impl IndexerService {
    pub fn new(
        soroban: SorobanClient,
        pool: PgPool,
        poll_interval: u64,
        contracts: Vec<ContractConfig>,
    ) -> Self {
        Self {
            soroban,
            pool,
            poll_interval,
            contracts,
        }
    }

    pub fn start(self: Arc<Self>, cancel: CancellationToken) {
        for contract in &self.contracts {
            let svc = Arc::clone(&self);
            let addr = contract.address.clone();
            let domain = contract.domain.clone();
            let interval = self.poll_interval;
            let cancel = cancel.clone();

            tokio::spawn(async move {
                tracing::info!("Starting indexer for {} ({})", domain, addr);
                let mut consecutive_errors = 0u32;

                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => {
                            tracing::info!("Indexer for {} shutting down", domain);
                            break;
                        }
                        result = svc.sync_contract(&addr) => {
                            match result {
                                Ok(_) => {
                                    consecutive_errors = 0;
                                }
                                Err(e) => {
                                    consecutive_errors += 1;
                                    let backoff = std::cmp::min(
                                        interval * 2u64.pow(consecutive_errors.min(6)),
                                        300, // max 5 minutes
                                    );
                                    tracing::error!(
                                        contract = %addr,
                                        consecutive_errors = consecutive_errors,
                                        backoff_secs = backoff,
                                        "Indexer error: {:?}", e
                                    );
                                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                                    continue;
                                }
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(interval)).await;
                }
            });
        }
    }

    async fn sync_contract(&self, contract_address: &str) -> anyhow::Result<()> {
        let last_height = self.get_last_synced_height(contract_address).await?;
        let events = self
            .soroban
            .get_events(contract_address, last_height, 100)
            .await?;

        if let Some(events_arr) = events
            .get("result")
            .and_then(|r| r.get("events"))
            .and_then(|e| e.as_array())
        {
            for event in events_arr {
                if let Err(e) = self.process_event(contract_address, event).await {
                    tracing::warn!("Failed to process event: {:?}", e);
                }
            }
        }

        Ok(())
    }

    async fn get_last_synced_height(&self, contract_address: &str) -> anyhow::Result<u64> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT COALESCE(synced_height, 0) FROM indexer_state WHERE contract_address = $1",
        )
        .bind(contract_address)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0 as u64).unwrap_or(0))
    }

    async fn process_event(
        &self,
        contract_address: &str,
        event: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let ledger = event.get("ledger").and_then(|l| l.as_u64()).unwrap_or(0);

        // Parse event topics to determine event type
        let topics = event
            .get("topics")
            .and_then(|t| t.as_array())
            .map(|t| t.to_vec())
            .unwrap_or_default();

        let event_type = topics.first().and_then(|t| t.as_str()).unwrap_or("unknown");

        // Get event data
        let data = event.get("data");

        // Route to appropriate handler based on event type
        match event_type {
            "farm_created" => {
                if let Some(data) = data {
                    self.handle_farm_created(data).await.ok();
                }
            }
            "investment_created" => {
                if let Some(data) = data {
                    self.handle_investment_created(data).await.ok();
                }
            }
            "investment_funded" => {
                if let Some(data) = data {
                    self.handle_investment_funded(data).await.ok();
                }
            }
            "escrow_created" | "escrow_completed" | "escrow_disputed" => {
                if let Some(data) = data {
                    self.handle_escrow_event(event_type, data).await.ok();
                }
            }
            "proposal_created" | "proposal_voted" | "proposal_executed" => {
                if let Some(data) = data {
                    self.handle_dao_event(event_type, data).await.ok();
                }
            }
            _ => {
                tracing::debug!(
                    event_type = event_type,
                    contract = contract_address,
                    "Unknown indexer event type"
                );
            }
        }

        // Always update synced height
        sqlx::query(
            r#"
            INSERT INTO indexer_state (contract_address, synced_height, last_synced_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (contract_address) DO UPDATE SET
                synced_height = GREATEST(indexer_state.synced_height, $2),
                last_synced_at = NOW()
            "#,
        )
        .bind(contract_address)
        .bind(ledger as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Event handlers - stub implementations
    // These would parse Soroban event data and update the appropriate tables
    async fn handle_farm_created(&self, _data: &serde_json::Value) -> anyhow::Result<()> {
        tracing::debug!("Processing farm_created event");
        // TODO: Parse farm data from event and insert into farms table
        Ok(())
    }

    async fn handle_investment_created(&self, _data: &serde_json::Value) -> anyhow::Result<()> {
        tracing::debug!("Processing investment_created event");
        // TODO: Parse investment data from event and insert into investments table
        Ok(())
    }

    async fn handle_investment_funded(&self, _data: &serde_json::Value) -> anyhow::Result<()> {
        tracing::debug!("Processing investment_funded event");
        // TODO: Parse funding data and update investments table
        Ok(())
    }

    async fn handle_escrow_event(
        &self,
        event_type: &str,
        _data: &serde_json::Value,
    ) -> anyhow::Result<()> {
        tracing::debug!("Processing escrow event: {}", event_type);
        // TODO: Parse escrow data and update escrows table based on event_type
        Ok(())
    }

    async fn handle_dao_event(
        &self,
        event_type: &str,
        _data: &serde_json::Value,
    ) -> anyhow::Result<()> {
        tracing::debug!("Processing DAO event: {}", event_type);
        // TODO: Parse proposal/vote data and update proposals/votes tables
        Ok(())
    }
}
