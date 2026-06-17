use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

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

#[derive(Debug, Deserialize, Validate)]
pub struct CreateFarm {
    #[validate(length(min = 1, max = 255, message = "Business name must be between 1 and 255 characters"))]
    pub business_name: String,

    #[validate(url(message = "Business image must be a valid URL"))]
    pub business_image: Option<String>,

    #[validate(length(max = 500, message = "Business location must not exceed 500 characters"))]
    pub business_location: Option<String>,

    #[validate(length(max = 255, message = "Business contact must not exceed 255 characters"))]
    pub business_contact: Option<String>,

    #[validate(email(message = "Business email must be a valid email address"))]
    pub business_email: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateFarm {
    #[validate(length(min = 1, max = 255, message = "Business name must be between 1 and 255 characters"))]
    pub business_name: Option<String>,

    #[validate(url(message = "Business image must be a valid URL"))]
    pub business_image: Option<String>,

    #[validate(length(max = 500, message = "Business location must not exceed 500 characters"))]
    pub business_location: Option<String>,

    #[validate(length(max = 255, message = "Business contact must not exceed 255 characters"))]
    pub business_contact: Option<String>,

    #[validate(email(message = "Business email must be a valid email address"))]
    pub business_email: Option<String>,
}
