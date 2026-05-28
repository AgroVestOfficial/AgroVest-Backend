use crate::error::ApiError;
use crate::models::review::{CreateReview, Review};
use sqlx::PgPool;

pub async fn get_product_reviews(
    pool: &PgPool,
    product_id: i32,
) -> Result<Vec<Review>, ApiError> {
    let reviews = sqlx::query_as::<_, Review>(
        "SELECT * FROM reviews WHERE product_id = $1 ORDER BY created_at DESC",
    )
    .bind(product_id)
    .fetch_all(pool)
    .await?;
    Ok(reviews)
}

pub async fn create_review(
    pool: &PgPool,
    reviewer: &str,
    product_id: i32,
    data: CreateReview,
) -> Result<Review, ApiError> {
    sqlx::query_as::<_, Review>(
        r#"
        INSERT INTO reviews (reviewer, review_text, product_id)
        VALUES ($1, $2, $3)
        ON CONFLICT (reviewer, product_id) DO UPDATE SET review_text = $2
        RETURNING *
        "#,
    )
    .bind(reviewer)
    .bind(&data.review_text)
    .bind(product_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))
}
