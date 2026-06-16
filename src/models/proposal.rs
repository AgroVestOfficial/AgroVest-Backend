use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Proposal {
    pub id: i32,
    pub proposal_id_onchain: Option<i32>,
    pub title: String,
    pub description: Option<String>,
    pub created_at_onchain: i64,
    pub ends_at: i64,
    pub required_votes: i64,
    pub proposer: String,
    pub executed: bool,
    pub is_challenged: bool,
    pub accept_votes: i64,
    pub reject_votes: i64,
    pub undecided_votes: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProposal {
    pub title: String,
    pub description: Option<String>,
    pub required_votes: i64,
    pub ends_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    pub vote: String,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteProposal {
    pub farm_id: i32,
    pub name: String,
    pub about: Option<String>,
    pub min_amount: i64,
    pub end_date: i64,
}
