use crate::db::redis::{get_cached, invalidate_pattern, set_cached};
use crate::error::ApiError;
use crate::models::product::{CreateProduct, Product, ProductFilter, UpdateProduct};
use crate::utils::pagination::{PaginatedResponse, PaginationParams};
use redis::aio::ConnectionManager;
use sqlx::PgPool;

pub async fn list_products(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    pagination: &PaginationParams,
    filter: &ProductFilter,
) -> Result<PaginatedResponse<Product>, ApiError> {
    let cache_key = format!(
        "products:list:{}:{}:{}:{}:{}",
        pagination.page(),
        pagination.per_page(),
        filter.category.as_deref().unwrap_or(""),
        filter
            .farm_id
            .map(|id| id.to_string())
            .as_deref()
            .unwrap_or(""),
        filter.sold.map(|s| s.to_string()).as_deref().unwrap_or("")
    );

    if let Some(cached) = get_cached::<PaginatedResponse<Product>>(redis, &cache_key).await {
        return Ok(cached);
    }

    let mut builder = sqlx::QueryBuilder::new("SELECT * FROM products WHERE 1=1");
    let mut count_builder = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM products WHERE 1=1");

    if let Some(ref category) = filter.category {
        builder.push(" AND category = ");
        builder.push_bind(category);
        count_builder.push(" AND category = ");
        count_builder.push_bind(category);
    }
    if let Some(farm_id) = filter.farm_id {
        builder.push(" AND farm_id = ");
        builder.push_bind(farm_id);
        count_builder.push(" AND farm_id = ");
        count_builder.push_bind(farm_id);
    }
    if let Some(sold) = filter.sold {
        builder.push(" AND sold = ");
        builder.push_bind(sold);
        count_builder.push(" AND sold = ");
        count_builder.push_bind(sold);
    }

    builder.push(" ORDER BY created_at DESC LIMIT ");
    builder.push_bind(pagination.per_page() as i64);
    builder.push(" OFFSET ");
    builder.push_bind(pagination.offset() as i64);

    let total: (i64,) = count_builder.build_query_as().fetch_one(pool).await?;
    let products = builder.build_query_as::<Product>().fetch_all(pool).await?;

    let result =
        PaginatedResponse::new(products, total.0, pagination.page(), pagination.per_page());

    // Cache for 30 seconds
    let _ = set_cached(redis, &cache_key, &result, 30).await;

    Ok(result)
}

pub async fn get_product(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    id: i32,
) -> Result<Product, ApiError> {
    let cache_key = format!("products:get:{}", id);

    if let Some(cached) = get_cached::<Product>(redis, &cache_key).await {
        return Ok(cached);
    }

    let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    // Cache for 60 seconds
    let _ = set_cached(redis, &cache_key, &product, 60).await;

    Ok(product)
}

pub async fn get_farm_products(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    farm_id: i32,
    pagination: &PaginationParams,
) -> Result<PaginatedResponse<Product>, ApiError> {
    let cache_key = format!(
        "products:farm:{}:{}:{}",
        farm_id,
        pagination.page(),
        pagination.per_page()
    );

    if let Some(cached) = get_cached::<PaginatedResponse<Product>>(redis, &cache_key).await {
        return Ok(cached);
    }

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

    let result =
        PaginatedResponse::new(products, total.0, pagination.page(), pagination.per_page());

    // Cache for 30 seconds
    let _ = set_cached(redis, &cache_key, &result, 30).await;

    Ok(result)
}

pub async fn create_product(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    owner: &str,
    data: CreateProduct,
) -> Result<Product, ApiError> {
    let product = sqlx::query_as::<_, Product>(
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
    .map_err(ApiError::Database)?;

    // Invalidate product caches
    let _ = invalidate_pattern(redis, "products:*").await;

    Ok(product)
}

pub async fn update_product(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    id: i32,
    owner: &str,
    data: UpdateProduct,
) -> Result<Product, ApiError> {
    let product = sqlx::query_as::<_, Product>(
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
    .ok_or(ApiError::NotFound)?;

    // Invalidate product caches
    let _ = invalidate_pattern(redis, "products:*").await;

    Ok(product)
}
