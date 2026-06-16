use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Dispute {
    pub id: i32,
    pub dispute_id_onchain: Option<i32>,
    pub challenge_id: i32,
    pub arbitrator: String,
    pub resolved: bool,
    pub ruling: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDispute {
    pub challenge_id: i32,
    pub arbitrator: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ResolveDispute {
    pub ruling: bool,
}
