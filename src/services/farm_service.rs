use crate::db::redis::{get_cached, invalidate_pattern, set_cached};
use crate::error::ApiError;
use crate::models::farm::{CreateFarm, Farm, UpdateFarm};
use crate::utils::pagination::{PaginatedResponse, PaginationParams};
use redis::aio::ConnectionManager;
use sqlx::PgPool;

pub async fn list_farms(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    pagination: &PaginationParams,
) -> Result<PaginatedResponse<Farm>, ApiError> {
    let cache_key = format!("farms:list:{}:{}", pagination.page(), pagination.per_page());

    if let Some(cached) = get_cached::<PaginatedResponse<Farm>>(redis, &cache_key).await {
        return Ok(cached);
    }

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM farms")
        .fetch_one(pool)
        .await?;

    let farms = sqlx::query_as::<_, Farm>(
        "SELECT * FROM farms ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(pagination.per_page() as i64)
    .bind(pagination.offset() as i64)
    .fetch_all(pool)
    .await?;

    let result = PaginatedResponse::new(farms, total.0, pagination.page(), pagination.per_page());

    // Cache for 60 seconds
    let _ = set_cached(redis, &cache_key, &result, 60).await;

    Ok(result)
}

pub async fn get_farm(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    id: i32,
) -> Result<Farm, ApiError> {
    let cache_key = format!("farms:get:{}", id);

    if let Some(cached) = get_cached::<Farm>(redis, &cache_key).await {
        return Ok(cached);
    }

    let farm = sqlx::query_as::<_, Farm>("SELECT * FROM farms WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    // Cache for 120 seconds
    let _ = set_cached(redis, &cache_key, &farm, 120).await;

    Ok(farm)
}

pub async fn get_farm_by_address(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    address: &str,
) -> Result<Farm, ApiError> {
    let cache_key = format!("farms:get_by_address:{}", address);

    if let Some(cached) = get_cached::<Farm>(redis, &cache_key).await {
        return Ok(cached);
    }

    let farm = sqlx::query_as::<_, Farm>("SELECT * FROM farms WHERE farmer_address = $1")
        .bind(address)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    // Cache for 120 seconds
    let _ = set_cached(redis, &cache_key, &farm, 120).await;

    Ok(farm)
}

pub async fn create_farm(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    farmer_address: &str,
    data: CreateFarm,
) -> Result<Farm, ApiError> {
    let farm = sqlx::query_as::<_, Farm>(
        r#"
        INSERT INTO farms (business_name, business_image, business_location, business_contact, business_email, farmer_address)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(&data.business_name)
    .bind(&data.business_image)
    .bind(&data.business_location)
    .bind(&data.business_contact)
    .bind(&data.business_email)
    .bind(farmer_address)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)?;

    // Invalidate list cache
    let _ = invalidate_pattern(redis, "farms:list:*").await;

    Ok(farm)
}

pub async fn update_farm(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    id: i32,
    farmer_address: &str,
    data: UpdateFarm,
) -> Result<Farm, ApiError> {
    let farm = sqlx::query_as::<_, Farm>(
        r#"
        UPDATE farms
        SET business_name = COALESCE($3, business_name),
            business_image = COALESCE($4, business_image),
            business_location = COALESCE($5, business_location),
            business_contact = COALESCE($6, business_contact),
            business_email = COALESCE($7, business_email),
            updated_at = NOW()
        WHERE id = $1 AND farmer_address = $2
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(farmer_address)
    .bind(&data.business_name)
    .bind(&data.business_image)
    .bind(&data.business_location)
    .bind(&data.business_contact)
    .bind(&data.business_email)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)?;

    // Invalidate both list and specific get caches
    let _ = invalidate_pattern(redis, "farms:*").await;

    Ok(farm)
}
