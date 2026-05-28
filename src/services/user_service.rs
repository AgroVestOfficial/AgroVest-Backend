use crate::error::ApiError;
use crate::models::user::{UpdateUser, User};
use sqlx::PgPool;

pub async fn get_user(pool: &PgPool, address: &str) -> Result<User, ApiError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE address = $1")
        .bind(address)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)
}

pub async fn upsert_user(pool: &PgPool, address: &str) -> Result<User, ApiError> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (address)
        VALUES ($1)
        ON CONFLICT (address) DO NOTHING
        RETURNING *
        "#,
    )
    .bind(address)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))
}

pub async fn update_user(
    pool: &PgPool,
    address: &str,
    update: UpdateUser,
) -> Result<User, ApiError> {
    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET display_name = COALESCE($2, display_name),
            bio = COALESCE($3, bio),
            avatar_url = COALESCE($4, avatar_url),
            updated_at = NOW()
        WHERE address = $1
        RETURNING *
        "#,
    )
    .bind(address)
    .bind(update.display_name)
    .bind(update.bio)
    .bind(update.avatar_url)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)
}
