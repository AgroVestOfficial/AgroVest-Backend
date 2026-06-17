use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CartItem {
    pub id: i32,
    pub user_address: String,
    pub product_id: i32,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AddToCart {
    #[validate(range(min = 1, message = "Product ID must be positive"))]
    pub product_id: i32,
}
