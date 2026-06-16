#![allow(dead_code)]

use crate::blockchain::soroban_client::SorobanClient;
use sqlx::PgPool;
use std::sync::Arc;

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

    pub fn start(self: Arc<Self>) {
        for contract in &self.contracts {
            let svc = Arc::clone(&self);
            let addr = contract.address.clone();
            let domain = contract.domain.clone();
            let interval = self.poll_interval;

            tokio::spawn(async move {
                tracing::info!("Starting indexer for {} ({})", domain, addr);
                loop {
                    if let Err(e) = svc.sync_contract(&addr).await {
                        tracing::error!("Indexer error for {}: {:?}", addr, e);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
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
        // Parse event and update DB based on topic
        // This is a placeholder - actual implementation depends on Soroban event format
        let ledger = event.get("ledger").and_then(|l| l.as_u64()).unwrap_or(0);

        // Update indexer state
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
}
