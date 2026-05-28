use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Farm {
    pub id: i32,
    pub farm_id_onchain: Option<i32>,
    pub business_name: String,
    pub business_image: Option<String>,
    pub business_location: Option<String>,
    pub business_contact: Option<String>,
    pub business_email: Option<String>,
    pub farmer_address: String,
    pub is_registered: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFarm {
    pub business_name: String,
    pub business_image: Option<String>,
    pub business_location: Option<String>,
    pub business_contact: Option<String>,
    pub business_email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFarm {
    pub business_name: Option<String>,
    pub business_image: Option<String>,
    pub business_location: Option<String>,
    pub business_contact: Option<String>,
    pub business_email: Option<String>,
}
