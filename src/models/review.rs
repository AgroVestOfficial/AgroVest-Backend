use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Review {
    pub id: i32,
    pub reviewer: String,
    pub review_text: String,
    pub product_id: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateReview {
    #[validate(length(
        min = 1,
        max = 5000,
        message = "Review text must be between 1 and 5000 characters"
    ))]
    pub review_text: String,
}
