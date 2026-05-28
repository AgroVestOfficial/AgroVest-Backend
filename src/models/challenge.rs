use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Challenge {
    pub id: i32,
    pub challenge_id_onchain: Option<i32>,
    pub proposal_id: i32,
    pub description: Option<String>,
    pub resolved: bool,
    pub challenger: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChallenge {
    pub proposal_id: i32,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveChallenge {
    pub valid: bool,
}
