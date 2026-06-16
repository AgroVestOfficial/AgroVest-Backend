use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CartItem {
    pub id: i32,
    pub user_address: String,
    pub product_id: i32,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AddToCart {
    pub product_id: i32,
}
