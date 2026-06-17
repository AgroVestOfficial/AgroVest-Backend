use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;
use crate::utils::validators::validate_stellar_address;

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

#[derive(Debug, Deserialize, Validate)]
pub struct CreateDispute {
    #[validate(range(min = 1, message = "Challenge ID must be positive"))]
    pub challenge_id: i32,

    #[validate(custom(function = "validate_stellar_address", message = "Invalid Stellar address format"))]
    pub arbitrator: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ResolveDispute {
    pub ruling: bool,
}
