use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: i32,
    pub product_id_onchain: Option<i32>,
    pub product_name: String,
    pub product_image: Option<String>,
    pub product_description: Option<String>,
    pub product_price: i64,
    pub product_owner: String,
    pub farm_id: Option<i32>,
    pub sold: bool,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProduct {
    pub product_name: String,
    pub product_image: Option<String>,
    pub product_description: Option<String>,
    pub product_price: i64,
    pub farm_id: Option<i32>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProduct {
    pub product_name: Option<String>,
    pub product_image: Option<String>,
    pub product_description: Option<String>,
    pub product_price: Option<i64>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProductFilter {
    pub category: Option<String>,
    pub farm_id: Option<i32>,
    pub sold: Option<bool>,
}
