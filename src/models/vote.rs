use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone)]
#[sqlx(type_name = "vote_type", rename_all = "snake_case")]
pub enum VoteType {
    Null,
    Accept,
    Reject,
    Undecided,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Vote {
    pub id: i32,
    pub proposal_id: i32,
    pub voter: String,
    pub voting_power: i64,
    pub vote_type: VoteType,
    pub created_at: DateTime<Utc>,
}
