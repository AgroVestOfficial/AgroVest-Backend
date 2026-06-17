use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

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

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProposal {
    #[validate(length(min = 1, max = 500, message = "Proposal title must be between 1 and 500 characters"))]
    pub title: String,

    #[validate(length(max = 5000, message = "Proposal description must not exceed 5000 characters"))]
    pub description: Option<String>,

    #[validate(range(min = 1, message = "Required votes must be at least 1 to ensure valid quorum"))]
    pub required_votes: i64,

    #[validate(range(min = 1000000000, message = "Proposal end date must be in the future (Unix timestamp >= 1000000000)"))]
    pub ends_at: i64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct VoteRequest {
    #[validate(length(min = 1, max = 50, message = "Vote must be between 1 and 50 characters"))]
    pub vote: String,
}

#[derive(Debug, Deserialize, Validate)]
#[allow(dead_code)]
pub struct ExecuteProposal {
    #[validate(range(min = 1, message = "Farm ID must be positive"))]
    pub farm_id: i32,

    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: String,

    #[validate(length(max = 5000, message = "Description must not exceed 5000 characters"))]
    pub about: Option<String>,

    #[validate(range(min = 1, message = "Minimum amount must be positive"))]
    pub min_amount: i64,

    #[validate(range(min = 1000000000, message = "End date must be in the future"))]
    pub end_date: i64,
}
