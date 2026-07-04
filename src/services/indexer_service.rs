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

        let topics = event
            .get("topics")
            .and_then(|t| t.as_array())
            .map(|t| t.to_vec())
            .unwrap_or_default();

        let event_type = topics.first().and_then(|t| t.as_str()).unwrap_or("unknown");

        let data = event.get("data");

        match event_type {
            "farm_created" | "farm_registered" => {
                if let Some(data) = data {
                    self.handle_farm_created(data).await?;
                }
            }
            "investment_created" => {
                if let Some(data) = data {
                    self.handle_investment_created(data).await?;
                }
            }
            "investment_funded" | "new_investment" => {
                if let Some(data) = data {
                    self.handle_investment_funded(data).await?;
                }
            }
            "escrow_created" | "escrow_completed" | "escrow_disputed" => {
                if let Some(data) = data {
                    self.handle_escrow_event(event_type, data).await?;
                }
            }
            "proposal_created" | "proposal_voted" | "proposal_executed" => {
                if let Some(data) = data {
                    self.handle_dao_event(event_type, data).await?;
                }
            }
            "product_added" => {
                if let Some(data) = data {
                    self.handle_product_added(data).await?;
                }
            }
            "product_reviewed" => {
                if let Some(data) = data {
                    self.handle_product_reviewed(data).await?;
                }
            }
            "challenge_created" | "challenge_resolved" => {
                if let Some(data) = data {
                    self.handle_challenge_event(event_type, data).await?;
                }
            }
            "dispute_initiated" | "dispute_resolved" => {
                if let Some(data) = data {
                    self.handle_dispute_event(event_type, data).await?;
                }
            }
            _ => {
                tracing::warn!(
                    event_type = event_type,
                    contract = contract_address,
                    "Unknown indexer event type"
                );
            }
        }

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

    async fn handle_farm_created(&self, data: &serde_json::Value) -> anyhow::Result<()> {
        let farm_id_onchain = data.get("farm_id").and_then(|v| v.as_i64());
        let business_name = data
            .get("business_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Farm");
        let business_image = data.get("business_image").and_then(|v| v.as_str());
        let business_location = data.get("business_location").and_then(|v| v.as_str());
        let business_contact = data.get("business_contact").and_then(|v| v.as_str());
        let business_email = data.get("business_email").and_then(|v| v.as_str());
        let farmer_address = data
            .get("farmer_address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("farm_created event missing farmer_address"))?;

        sqlx::query(
            r#"
            INSERT INTO users (address) VALUES ($1)
            ON CONFLICT (address) DO NOTHING
            "#,
        )
        .bind(farmer_address)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO farms (farm_id_onchain, business_name, business_image, business_location, business_contact, business_email, farmer_address)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (farm_id_onchain) DO UPDATE SET
                business_name = EXCLUDED.business_name,
                business_image = EXCLUDED.business_image,
                business_location = EXCLUDED.business_location,
                business_contact = EXCLUDED.business_contact,
                business_email = EXCLUDED.business_email,
                updated_at = NOW()
            "#,
        )
        .bind(farm_id_onchain.map(|v| v as i32))
        .bind(business_name)
        .bind(business_image)
        .bind(business_location)
        .bind(business_contact)
        .bind(business_email)
        .bind(farmer_address)
        .execute(&self.pool)
        .await?;

        tracing::info!("Indexed farm_created for onchain_id {:?}", farm_id_onchain);
        Ok(())
    }

    async fn handle_investment_created(&self, data: &serde_json::Value) -> anyhow::Result<()> {
        let investment_id_onchain = data.get("investment_id").and_then(|v| v.as_i64());
        let farm_id_onchain = data.get("farm_id").and_then(|v| v.as_i64());
        let name = data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Investment");
        let image = data.get("image").and_then(|v| v.as_str());
        let about = data.get("about").and_then(|v| v.as_str());
        let owner = data
            .get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("investment_created event missing owner"))?;
        let min_amount = data.get("min_amount").and_then(|v| v.as_i64()).unwrap_or(0);
        let start_date = data.get("start_date").and_then(|v| v.as_i64()).unwrap_or(0);
        let end_date = data.get("end_date").and_then(|v| v.as_i64()).unwrap_or(0);

        sqlx::query(
            r#"
            INSERT INTO users (address) VALUES ($1)
            ON CONFLICT (address) DO NOTHING
            "#,
        )
        .bind(owner)
        .execute(&self.pool)
        .await?;

        let farm_id: Option<i32> = if let Some(fk) = farm_id_onchain {
            let row: Option<(i32,)> =
                sqlx::query_as("SELECT id FROM farms WHERE farm_id_onchain = $1")
                    .bind(fk as i32)
                    .fetch_optional(&self.pool)
                    .await?;
            row.map(|r| r.0)
        } else {
            None
        };

        sqlx::query(
            r#"
            INSERT INTO investments (investment_id_onchain, farm_id, image, name, about, owner, min_amount, start_date, end_date)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (investment_id_onchain) DO UPDATE SET
                farm_id = EXCLUDED.farm_id,
                image = EXCLUDED.image,
                name = EXCLUDED.name,
                about = EXCLUDED.about,
                owner = EXCLUDED.owner,
                min_amount = EXCLUDED.min_amount,
                start_date = EXCLUDED.start_date,
                end_date = EXCLUDED.end_date,
                updated_at = NOW()
            "#,
        )
        .bind(investment_id_onchain.map(|v| v as i32))
        .bind(farm_id)
        .bind(image)
        .bind(name)
        .bind(about)
        .bind(owner)
        .bind(min_amount)
        .bind(start_date)
        .bind(end_date)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            "Indexed investment_created for onchain_id {:?}",
            investment_id_onchain
        );
        Ok(())
    }

    async fn handle_investment_funded(&self, data: &serde_json::Value) -> anyhow::Result<()> {
        let investment_id_onchain = data
            .get("investment_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow::anyhow!("investment_funded event missing investment_id"))?;
        let amount = data.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
        let investor_address = data.get("investor").and_then(|v| v.as_str());

        if let Some(addr) = investor_address {
            sqlx::query("INSERT INTO users (address) VALUES ($1) ON CONFLICT (address) DO NOTHING")
                .bind(addr)
                .execute(&self.pool)
                .await?;
        }

        let investment_row: Option<(i32, i32)> =
            sqlx::query_as("SELECT id, farm_id FROM investments WHERE investment_id_onchain = $1")
                .bind(investment_id_onchain as i32)
                .fetch_optional(&self.pool)
                .await?;

        if let Some((investment_id, farm_id)) = investment_row {
            sqlx::query(
                r#"
                UPDATE investments
                SET amount_raised = amount_raised + $2,
                    farm_investor_count = farm_investor_count + 1,
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(investment_id)
            .bind(amount)
            .execute(&self.pool)
            .await?;

            if let Some(addr) = investor_address {
                let existing: Option<(i64,)> = sqlx::query_as(
                    "SELECT amount FROM investors WHERE investment_id = $1 AND investor_address = $2",
                )
                .bind(investment_id)
                .bind(addr)
                .fetch_optional(&self.pool)
                .await?;

                if existing.is_some() {
                    sqlx::query(
                        "UPDATE investors SET amount = amount + $3 WHERE investment_id = $1 AND investor_address = $2",
                    )
                    .bind(investment_id)
                    .bind(addr)
                    .bind(amount)
                    .execute(&self.pool)
                    .await?;
                } else {
                    sqlx::query(
                        r#"
                        INSERT INTO investors (farm_id, investment_id, investor_address, amount)
                        VALUES ($1, $2, $3, $4)
                        "#,
                    )
                    .bind(farm_id)
                    .bind(investment_id)
                    .bind(addr)
                    .bind(amount)
                    .execute(&self.pool)
                    .await?;
                }
            }
        } else {
            tracing::warn!(
                "investment_funded for unknown onchain_id {}",
                investment_id_onchain
            );
        }

        tracing::info!(
            "Indexed investment_funded for onchain_id {}, amount {}",
            investment_id_onchain,
            amount
        );
        Ok(())
    }

    async fn handle_escrow_event(
        &self,
        event_type: &str,
        data: &serde_json::Value,
    ) -> anyhow::Result<()> {
        let escrow_id_onchain = data.get("escrow_id").and_then(|v| v.as_i64());
        let buyer = data.get("buyer").and_then(|v| v.as_str());
        let farmer = data.get("farmer").and_then(|v| v.as_str());
        let amount = data.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
        let order_id = data.get("order_id").and_then(|v| v.as_i64()).unwrap_or(0);

        let status = match event_type {
            "escrow_completed" => "complete",
            "escrow_disputed" => "dispute",
            _ => "awaiting_delivery",
        };

        for addr in [buyer, farmer].into_iter().flatten() {
            sqlx::query("INSERT INTO users (address) VALUES ($1) ON CONFLICT (address) DO NOTHING")
                .bind(addr)
                .execute(&self.pool)
                .await?;
        }

        sqlx::query(
            r#"
            INSERT INTO escrows (escrow_id_onchain, buyer, farmer, amount, status, order_id)
            VALUES ($1, $2, $3, $4, $5::escrow_status, $6)
            ON CONFLICT (escrow_id_onchain) DO UPDATE SET
                status = EXCLUDED.status,
                updated_at = NOW()
            "#,
        )
        .bind(escrow_id_onchain.map(|v| v as i32))
        .bind(buyer.unwrap_or(""))
        .bind(farmer.unwrap_or(""))
        .bind(amount)
        .bind(status)
        .bind(order_id as i32)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            "Indexed {} for onchain_id {:?}",
            event_type,
            escrow_id_onchain
        );
        Ok(())
    }

    async fn handle_dao_event(
        &self,
        event_type: &str,
        data: &serde_json::Value,
    ) -> anyhow::Result<()> {
        match event_type {
            "proposal_created" => {
                let proposal_id_onchain = data.get("proposal_id").and_then(|v| v.as_i64());
                let title = data
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled Proposal");
                let description = data.get("description").and_then(|v| v.as_str());
                let proposer = data
                    .get("proposer")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("proposal_created event missing proposer"))?;
                let required_votes = data
                    .get("required_votes")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1);
                let ends_at = data.get("ends_at").and_then(|v| v.as_i64()).unwrap_or(0);
                let created_at_onchain =
                    data.get("created_at").and_then(|v| v.as_i64()).unwrap_or(0);

                sqlx::query(
                    "INSERT INTO users (address) VALUES ($1) ON CONFLICT (address) DO NOTHING",
                )
                .bind(proposer)
                .execute(&self.pool)
                .await?;

                sqlx::query(
                    r#"
                    INSERT INTO proposals (proposal_id_onchain, title, description, created_at_onchain, ends_at, required_votes, proposer)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (proposal_id_onchain) DO UPDATE SET
                        title = EXCLUDED.title,
                        description = EXCLUDED.description,
                        ends_at = EXCLUDED.ends_at,
                        required_votes = EXCLUDED.required_votes,
                        updated_at = NOW()
                    "#,
                )
                .bind(proposal_id_onchain.map(|v| v as i32))
                .bind(title)
                .bind(description)
                .bind(created_at_onchain)
                .bind(ends_at)
                .bind(required_votes)
                .bind(proposer)
                .execute(&self.pool)
                .await?;

                tracing::info!(
                    "Indexed proposal_created for onchain_id {:?}",
                    proposal_id_onchain
                );
            }
            "proposal_voted" => {
                let proposal_id_onchain = data
                    .get("proposal_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow::anyhow!("proposal_voted event missing proposal_id"))?;
                let voter = data
                    .get("voter")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("proposal_voted event missing voter"))?;
                let vote_type = data
                    .get("vote_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("undecided");
                let voting_power = data
                    .get("voting_power")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1);

                let row: Option<(i32,)> =
                    sqlx::query_as("SELECT id FROM proposals WHERE proposal_id_onchain = $1")
                        .bind(proposal_id_onchain as i32)
                        .fetch_optional(&self.pool)
                        .await?;

                if let Some((proposal_id,)) = row {
                    let vt = match vote_type {
                        "accept" => "accept",
                        "reject" => "reject",
                        _ => "undecided",
                    };

                    sqlx::query(
                        r#"
                        INSERT INTO votes (proposal_id, voter, voting_power, vote_type)
                        VALUES ($1, $2, $3, $4::vote_type)
                        ON CONFLICT (proposal_id, voter) DO UPDATE SET
                            voting_power = EXCLUDED.voting_power,
                            vote_type = EXCLUDED.vote_type
                        "#,
                    )
                    .bind(proposal_id)
                    .bind(voter)
                    .bind(voting_power)
                    .bind(vt)
                    .execute(&self.pool)
                    .await?;

                    let vote_delta: &str = match vt {
                        "accept" => "accept_votes",
                        "reject" => "reject_votes",
                        _ => "undecided_votes",
                    };

                    sqlx::query(&format!(
                        r#"
                        UPDATE proposals
                        SET {col} = {col} + $2, updated_at = NOW()
                        WHERE id = $1
                        "#,
                        col = vote_delta
                    ))
                    .bind(proposal_id)
                    .bind(voting_power)
                    .execute(&self.pool)
                    .await?;

                    tracing::info!(
                        "Indexed proposal_voted for onchain_id {}, voter {}",
                        proposal_id_onchain,
                        voter
                    );
                } else {
                    tracing::warn!(
                        "proposal_voted for unknown onchain_id {}",
                        proposal_id_onchain
                    );
                }
            }
            "proposal_executed" => {
                let proposal_id_onchain = data
                    .get("proposal_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| {
                        anyhow::anyhow!("proposal_executed event missing proposal_id")
                    })?;

                sqlx::query(
                    r#"
                    UPDATE proposals
                    SET executed = true, updated_at = NOW()
                    WHERE proposal_id_onchain = $1
                    "#,
                )
                .bind(proposal_id_onchain as i32)
                .execute(&self.pool)
                .await?;

                tracing::info!(
                    "Indexed proposal_executed for onchain_id {}",
                    proposal_id_onchain
                );
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_product_added(&self, data: &serde_json::Value) -> anyhow::Result<()> {
        let product_id_onchain = data.get("product_id").and_then(|v| v.as_i64());
        let name = data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Product");
        let image = data.get("image").and_then(|v| v.as_str());
        let description = data.get("description").and_then(|v| v.as_str());
        let price = data.get("price").and_then(|v| v.as_i64()).unwrap_or(0);
        let owner = data
            .get("owner")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("product_added event missing owner"))?;
        let farm_id_onchain = data.get("farm_id").and_then(|v| v.as_i64());
        let category = data.get("category").and_then(|v| v.as_str());

        sqlx::query("INSERT INTO users (address) VALUES ($1) ON CONFLICT (address) DO NOTHING")
            .bind(owner)
            .execute(&self.pool)
            .await?;

        let farm_id: Option<i32> = if let Some(fk) = farm_id_onchain {
            let row: Option<(i32,)> =
                sqlx::query_as("SELECT id FROM farms WHERE farm_id_onchain = $1")
                    .bind(fk as i32)
                    .fetch_optional(&self.pool)
                    .await?;
            row.map(|r| r.0)
        } else {
            None
        };

        sqlx::query(
            r#"
            INSERT INTO products (product_id_onchain, product_name, product_image, product_description, product_price, product_owner, farm_id, category)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (product_id_onchain) DO UPDATE SET
                product_name = EXCLUDED.product_name,
                product_image = EXCLUDED.product_image,
                product_description = EXCLUDED.product_description,
                product_price = EXCLUDED.product_price,
                category = EXCLUDED.category,
                updated_at = NOW()
            "#,
        )
        .bind(product_id_onchain.map(|v| v as i32))
        .bind(name)
        .bind(image)
        .bind(description)
        .bind(price)
        .bind(owner)
        .bind(farm_id)
        .bind(category)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            "Indexed product_added for onchain_id {:?}",
            product_id_onchain
        );
        Ok(())
    }

    async fn handle_product_reviewed(&self, data: &serde_json::Value) -> anyhow::Result<()> {
        let reviewer = data
            .get("reviewer")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("product_reviewed event missing reviewer"))?;
        let product_id_onchain = data
            .get("product_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow::anyhow!("product_reviewed event missing product_id"))?;
        let review_text = data
            .get("review_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        sqlx::query("INSERT INTO users (address) VALUES ($1) ON CONFLICT (address) DO NOTHING")
            .bind(reviewer)
            .execute(&self.pool)
            .await?;

        let product_row: Option<(i32,)> =
            sqlx::query_as("SELECT id FROM products WHERE product_id_onchain = $1")
                .bind(product_id_onchain as i32)
                .fetch_optional(&self.pool)
                .await?;

        if let Some((product_id,)) = product_row {
            sqlx::query(
                r#"
                INSERT INTO reviews (reviewer, review_text, product_id)
                VALUES ($1, $2, $3)
                ON CONFLICT (reviewer, product_id) DO UPDATE SET
                    review_text = EXCLUDED.review_text
                "#,
            )
            .bind(reviewer)
            .bind(review_text)
            .bind(product_id)
            .execute(&self.pool)
            .await?;

            tracing::info!(
                "Indexed product_reviewed for product onchain_id {}",
                product_id_onchain
            );
        } else {
            tracing::warn!(
                "product_reviewed for unknown product onchain_id {}",
                product_id_onchain
            );
        }

        Ok(())
    }

    async fn handle_challenge_event(
        &self,
        event_type: &str,
        data: &serde_json::Value,
    ) -> anyhow::Result<()> {
        match event_type {
            "challenge_created" => {
                let challenge_id_onchain = data.get("challenge_id").and_then(|v| v.as_i64());
                let proposal_id_onchain = data.get("proposal_id").and_then(|v| v.as_i64());
                let description = data.get("description").and_then(|v| v.as_str());
                let challenger = data
                    .get("challenger")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("challenge_created event missing challenger"))?;

                sqlx::query(
                    "INSERT INTO users (address) VALUES ($1) ON CONFLICT (address) DO NOTHING",
                )
                .bind(challenger)
                .execute(&self.pool)
                .await?;

                let proposal_id: Option<i32> = if let Some(pk) = proposal_id_onchain {
                    let row: Option<(i32,)> =
                        sqlx::query_as("SELECT id FROM proposals WHERE proposal_id_onchain = $1")
                            .bind(pk as i32)
                            .fetch_optional(&self.pool)
                            .await?;
                    row.map(|r| r.0)
                } else {
                    None
                };

                if let Some(pid) = proposal_id {
                    sqlx::query(
                        r#"
                        INSERT INTO challenges (challenge_id_onchain, proposal_id, description, challenger)
                        VALUES ($1, $2, $3, $4)
                        ON CONFLICT (challenge_id_onchain) DO UPDATE SET
                            description = EXCLUDED.description,
                            resolved = false
                        "#,
                    )
                    .bind(challenge_id_onchain.map(|v| v as i32))
                    .bind(pid)
                    .bind(description)
                    .bind(challenger)
                    .execute(&self.pool)
                    .await?;

                    tracing::info!(
                        "Indexed challenge_created for onchain_id {:?}",
                        challenge_id_onchain
                    );
                }
            }
            "challenge_resolved" => {
                let challenge_id_onchain = data
                    .get("challenge_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| {
                        anyhow::anyhow!("challenge_resolved event missing challenge_id")
                    })?;

                sqlx::query(
                    r#"
                    UPDATE challenges
                    SET resolved = true
                    WHERE challenge_id_onchain = $1
                    "#,
                )
                .bind(challenge_id_onchain as i32)
                .execute(&self.pool)
                .await?;

                tracing::info!(
                    "Indexed challenge_resolved for onchain_id {}",
                    challenge_id_onchain
                );
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_dispute_event(
        &self,
        event_type: &str,
        data: &serde_json::Value,
    ) -> anyhow::Result<()> {
        match event_type {
            "dispute_initiated" => {
                let dispute_id_onchain = data.get("dispute_id").and_then(|v| v.as_i64());
                let challenge_id_onchain = data.get("challenge_id").and_then(|v| v.as_i64());
                let caller = data
                    .get("caller")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("dispute_initiated event missing caller"))?;

                sqlx::query(
                    "INSERT INTO users (address) VALUES ($1) ON CONFLICT (address) DO NOTHING",
                )
                .bind(caller)
                .execute(&self.pool)
                .await?;

                let challenge_id: Option<i32> = if let Some(ck) = challenge_id_onchain {
                    let row: Option<(i32,)> =
                        sqlx::query_as("SELECT id FROM challenges WHERE challenge_id_onchain = $1")
                            .bind(ck as i32)
                            .fetch_optional(&self.pool)
                            .await?;
                    row.map(|r| r.0)
                } else {
                    None
                };

                if let Some(cid) = challenge_id {
                    sqlx::query(
                        r#"
                        INSERT INTO disputes (dispute_id_onchain, challenge_id, arbitrator)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (dispute_id_onchain) DO UPDATE SET
                            resolved = false,
                            ruling = false
                        "#,
                    )
                    .bind(dispute_id_onchain.map(|v| v as i32))
                    .bind(cid)
                    .bind(caller)
                    .execute(&self.pool)
                    .await?;

                    tracing::info!(
                        "Indexed dispute_initiated for onchain_id {:?}",
                        dispute_id_onchain
                    );
                }
            }
            "dispute_resolved" => {
                let dispute_id_onchain = data
                    .get("dispute_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow::anyhow!("dispute_resolved event missing dispute_id"))?;
                let ruling = data
                    .get("ruling")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                sqlx::query(
                    r#"
                    UPDATE disputes
                    SET resolved = true, ruling = $2
                    WHERE dispute_id_onchain = $1
                    "#,
                )
                .bind(dispute_id_onchain as i32)
                .bind(ruling)
                .execute(&self.pool)
                .await?;

                tracing::info!(
                    "Indexed dispute_resolved for onchain_id {}",
                    dispute_id_onchain
                );
            }
            _ => {}
        }

        Ok(())
    }
}
