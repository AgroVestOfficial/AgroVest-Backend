use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

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

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProduct {
    #[validate(length(min = 1, max = 255, message = "Product name must be between 1 and 255 characters"))]
    pub product_name: String,

    #[validate(url(message = "Product image must be a valid URL"))]
    pub product_image: Option<String>,

    #[validate(length(max = 5000, message = "Product description must not exceed 5000 characters"))]
    pub product_description: Option<String>,

    #[validate(range(min = 1, message = "Product price must be positive (at least 1 cent in smallest unit)"))]
    pub product_price: i64,

    pub farm_id: Option<i32>,

    #[validate(length(max = 100, message = "Category must not exceed 100 characters"))]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProduct {
    #[validate(length(min = 1, max = 255, message = "Product name must be between 1 and 255 characters"))]
    pub product_name: Option<String>,

    #[validate(url(message = "Product image must be a valid URL"))]
    pub product_image: Option<String>,

    #[validate(length(max = 5000, message = "Product description must not exceed 5000 characters"))]
    pub product_description: Option<String>,

    #[validate(range(min = 1, message = "Product price must be positive (at least 1 cent in smallest unit)"))]
    pub product_price: Option<i64>,

    #[validate(length(max = 100, message = "Category must not exceed 100 characters"))]
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProductFilter {
    pub category: Option<String>,
    pub farm_id: Option<i32>,
    pub sold: Option<bool>,
}
