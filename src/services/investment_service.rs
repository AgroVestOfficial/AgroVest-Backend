use crate::db::redis::{get_cached, invalidate_pattern, set_cached};
use crate::error::ApiError;
use crate::models::investment::{CreateInvestment, Investment};
use crate::models::investor::Investor;
use crate::utils::pagination::{PaginatedResponse, PaginationParams};
use redis::aio::ConnectionManager;
use sqlx::PgPool;

pub async fn list_investments(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    pagination: &PaginationParams,
    active_only: bool,
) -> Result<PaginatedResponse<Investment>, ApiError> {
    let cache_key = format!(
        "investments:list:{}:{}:{}",
        pagination.page(),
        pagination.per_page(),
        if active_only { "active" } else { "all" }
    );

    if let Some(cached) = get_cached::<PaginatedResponse<Investment>>(redis, &cache_key).await {
        return Ok(cached);
    }

    let (count_sql, sql) = if active_only {
        (
            "SELECT COUNT(*) FROM investments WHERE is_active = true",
            "SELECT * FROM investments WHERE is_active = true ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
    } else {
        (
            "SELECT COUNT(*) FROM investments",
            "SELECT * FROM investments ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
    };

    let total: (i64,) = sqlx::query_as(count_sql).fetch_one(pool).await?;

    let investments = sqlx::query_as::<_, Investment>(sql)
        .bind(pagination.per_page() as i64)
        .bind(pagination.offset() as i64)
        .fetch_all(pool)
        .await?;

    let result = PaginatedResponse::new(
        investments,
        total.0,
        pagination.page(),
        pagination.per_page(),
    );

    // Cache for 15 seconds (investments change frequently)
    let _ = set_cached(redis, &cache_key, &result, 15).await;

    Ok(result)
}

pub async fn get_investment(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    id: i32,
) -> Result<Investment, ApiError> {
    let cache_key = format!("investments:get:{}", id);

    if let Some(cached) = get_cached::<Investment>(redis, &cache_key).await {
        return Ok(cached);
    }

    let investment = sqlx::query_as::<_, Investment>("SELECT * FROM investments WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    // Cache for 15 seconds
    let _ = set_cached(redis, &cache_key, &investment, 15).await;

    Ok(investment)
}

pub async fn get_investors(pool: &PgPool, investment_id: i32) -> Result<Vec<Investor>, ApiError> {
    let investors = sqlx::query_as::<_, Investor>(
        "SELECT * FROM investors WHERE investment_id = $1 ORDER BY created_at DESC",
    )
    .bind(investment_id)
    .fetch_all(pool)
    .await?;
    Ok(investors)
}

pub async fn create_investment(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    owner: &str,
    data: CreateInvestment,
) -> Result<Investment, ApiError> {
    let investment = sqlx::query_as::<_, Investment>(
        r#"
        INSERT INTO investments (farm_id, image, name, about, owner, min_amount, start_date, end_date)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(data.farm_id)
    .bind(&data.image)
    .bind(&data.name)
    .bind(&data.about)
    .bind(owner)
    .bind(data.min_amount)
    .bind(chrono::Utc::now().timestamp())
    .bind(data.end_date)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)?;

    // Invalidate investment caches
    let _ = invalidate_pattern(redis, "investments:*").await;

    Ok(investment)
}

pub async fn invest(
    pool: &PgPool,
    redis: &mut ConnectionManager,
    investor_address: &str,
    investment_id: i32,
    amount: i64,
) -> Result<Investor, ApiError> {
    let mut tx = pool.begin().await.map_err(ApiError::Database)?;

    let investment = sqlx::query_as::<_, Investment>(
        "SELECT * FROM investments WHERE id = $1 AND is_active = true FOR UPDATE",
    )
    .bind(investment_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;

    if amount < investment.min_amount {
        return Err(ApiError::BadRequest("Amount below minimum".into()));
    }

    let investor = sqlx::query_as::<_, Investor>(
        r#"
        INSERT INTO investors (farm_id, investment_id, investor_address, amount)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(investment.farm_id)
    .bind(investment_id)
    .bind(investor_address)
    .bind(amount)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e {
        // 23503 = foreign_key_violation: the referenced farm does not exist.
        sqlx::Error::Database(ref db_err) if db_err.code().as_deref() == Some("23503") => {
            ApiError::BadRequest("Invalid farm_id: farm does not exist".into())
        }
        other => ApiError::Database(other),
    })?;

    sqlx::query(
        "UPDATE investments SET amount_raised = amount_raised + $1, farm_investor_count = farm_investor_count + 1 WHERE id = $2",
    )
    .bind(amount)
    .bind(investment_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await.map_err(ApiError::Database)?;

    // Invalidate investment caches since balance changed
    let _ = invalidate_pattern(redis, "investments:*").await;

    Ok(investor)
}
