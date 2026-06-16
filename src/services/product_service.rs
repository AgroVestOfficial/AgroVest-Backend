use crate::error::ApiError;
use crate::models::product::{CreateProduct, Product, ProductFilter, UpdateProduct};
use crate::utils::pagination::{PaginatedResponse, PaginationParams};
use sqlx::PgPool;

pub async fn list_products(
    pool: &PgPool,
    pagination: &PaginationParams,
    filter: &ProductFilter,
) -> Result<PaginatedResponse<Product>, ApiError> {
    let mut query = String::from("SELECT * FROM products WHERE 1=1");
    let mut count_query = String::from("SELECT COUNT(*) FROM products WHERE 1=1");
    let mut bind_idx = 1;

    if filter.category.is_some() {
        query.push_str(&format!(" AND category = ${}", bind_idx));
        count_query.push_str(&format!(" AND category = ${}", bind_idx));
        bind_idx += 1;
    }
    if filter.farm_id.is_some() {
        query.push_str(&format!(" AND farm_id = ${}", bind_idx));
        count_query.push_str(&format!(" AND farm_id = ${}", bind_idx));
        bind_idx += 1;
    }
    if filter.sold.is_some() {
        query.push_str(&format!(" AND sold = ${}", bind_idx));
        count_query.push_str(&format!(" AND sold = ${}", bind_idx));
        bind_idx += 1;
    }

    query.push_str(&format!(
        " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
        bind_idx,
        bind_idx + 1
    ));

    let total: (i64,) = sqlx::query_as(&count_query).fetch_one(pool).await?;

    let products = sqlx::query_as::<_, Product>(&query).fetch_all(pool).await?;

    Ok(PaginatedResponse::new(
        products,
        total.0,
        pagination.page(),
        pagination.per_page(),
    ))
}

pub async fn get_product(pool: &PgPool, id: i32) -> Result<Product, ApiError> {
    sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)
}

pub async fn get_farm_products(
    pool: &PgPool,
    farm_id: i32,
    pagination: &PaginationParams,
) -> Result<PaginatedResponse<Product>, ApiError> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products WHERE farm_id = $1")
        .bind(farm_id)
        .fetch_one(pool)
        .await?;

    let products = sqlx::query_as::<_, Product>(
        "SELECT * FROM products WHERE farm_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(farm_id)
    .bind(pagination.per_page() as i64)
    .bind(pagination.offset() as i64)
    .fetch_all(pool)
    .await?;

    Ok(PaginatedResponse::new(
        products,
        total.0,
        pagination.page(),
        pagination.per_page(),
    ))
}

pub async fn create_product(
    pool: &PgPool,
    owner: &str,
    data: CreateProduct,
) -> Result<Product, ApiError> {
    sqlx::query_as::<_, Product>(
        r#"
        INSERT INTO products (product_name, product_image, product_description, product_price, product_owner, farm_id, category)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(&data.product_name)
    .bind(&data.product_image)
    .bind(&data.product_description)
    .bind(data.product_price)
    .bind(owner)
    .bind(data.farm_id)
    .bind(&data.category)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Database(e))
}

pub async fn update_product(
    pool: &PgPool,
    id: i32,
    owner: &str,
    data: UpdateProduct,
) -> Result<Product, ApiError> {
    sqlx::query_as::<_, Product>(
        r#"
        UPDATE products
        SET product_name = COALESCE($3, product_name),
            product_image = COALESCE($4, product_image),
            product_description = COALESCE($5, product_description),
            product_price = COALESCE($6, product_price),
            category = COALESCE($7, category),
            updated_at = NOW()
        WHERE id = $1 AND product_owner = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(owner)
    .bind(&data.product_name)
    .bind(&data.product_image)
    .bind(&data.product_description)
    .bind(data.product_price)
    .bind(&data.category)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)
}
