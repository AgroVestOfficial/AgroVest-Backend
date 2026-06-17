use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

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

#[derive(Debug, Deserialize, Validate)]
pub struct CreateChallenge {
    #[validate(range(min = 1, message = "Proposal ID must be positive"))]
    pub proposal_id: i32,

    #[validate(length(max = 5000, message = "Challenge description must not exceed 5000 characters"))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ResolveChallenge {
    pub valid: bool,
}
