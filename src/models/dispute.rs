use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

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
    // arbitrator removed — it is taken from the authenticated user's JWT
    // (auth.address) in the service layer, never from the request body.
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ResolveDispute {
    pub ruling: bool,
}
