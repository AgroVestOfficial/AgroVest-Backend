use crate::error::ApiError;
use crate::models::cart_item::CartItem;
use crate::models::product::Product;
use sqlx::PgPool;

pub async fn get_cart(pool: &PgPool, user_address: &str) -> Result<Vec<Product>, ApiError> {
    let products = sqlx::query_as::<_, Product>(
        r#"
        SELECT p.* FROM products p
        INNER JOIN cart_items ci ON ci.product_id = p.id
        WHERE ci.user_address = $1
        ORDER BY ci.added_at DESC
        "#,
    )
    .bind(user_address)
    .fetch_all(pool)
    .await?;
    Ok(products)
}

pub async fn add_to_cart(
    pool: &PgPool,
    user_address: &str,
    product_id: i32,
) -> Result<CartItem, ApiError> {
    sqlx::query_as::<_, CartItem>(
        r#"
        INSERT INTO cart_items (user_address, product_id)
        VALUES ($1, $2)
        ON CONFLICT (user_address, product_id) DO NOTHING
        RETURNING *
        "#,
    )
    .bind(user_address)
    .bind(product_id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::Conflict("Item already in cart".into()))
}

pub async fn remove_from_cart(
    pool: &PgPool,
    user_address: &str,
    product_id: i32,
) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM cart_items WHERE user_address = $1 AND product_id = $2")
        .bind(user_address)
        .bind(product_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn clear_cart(pool: &PgPool, user_address: &str) -> Result<(), ApiError> {
    sqlx::query("DELETE FROM cart_items WHERE user_address = $1")
        .bind(user_address)
        .execute(pool)
        .await?;
    Ok(())
}
