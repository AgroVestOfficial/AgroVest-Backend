use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

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

#[derive(Debug, Deserialize, Validate)]
pub struct CreateInvestment {
    #[validate(range(min = 1, message = "Farm ID must be positive"))]
    pub farm_id: i32,

    #[validate(url(message = "Image must be a valid URL"))]
    pub image: Option<String>,

    #[validate(length(min = 1, max = 255, message = "Investment name must be between 1 and 255 characters"))]
    pub name: String,

    #[validate(length(max = 5000, message = "Investment description must not exceed 5000 characters"))]
    pub about: Option<String>,

    #[validate(range(min = 1, message = "Minimum investment amount must be positive"))]
    pub min_amount: i64,

    #[validate(range(min = 1000000000, message = "End date must be in the future (Unix timestamp >= 1000000000)"))]
    pub end_date: i64,
}
