use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Investor {
    pub id: i32,
    pub investor_id_onchain: Option<i32>,
    pub farm_id: i32,
    pub investment_id: i32,
    pub investor_address: String,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvestmentEntry {
    pub amount: i64,
}
