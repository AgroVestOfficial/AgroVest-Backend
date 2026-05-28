use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Investment {
    pub id: i32,
    pub investment_id_onchain: Option<i32>,
    pub farm_id: i32,
    pub image: Option<String>,
    pub name: String,
    pub about: Option<String>,
    pub owner: String,
    pub min_amount: i64,
    pub amount_raised: i64,
    pub start_date: i64,
    pub end_date: i64,
    pub farm_investor_count: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvestment {
    pub farm_id: i32,
    pub image: Option<String>,
    pub name: String,
    pub about: Option<String>,
    pub min_amount: i64,
    pub end_date: i64,
}
