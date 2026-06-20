use crate::error::ApiError;
use crate::models::escrow::{CreateEscrow, Escrow, EscrowFilter, EscrowStatus};
use crate::utils::pagination::{PaginatedResponse, PaginationParams};
use sqlx::PgPool;

pub async fn list_escrows(
    pool: &PgPool,
    user_address: &str,
    pagination: &PaginationParams,
    filter: &EscrowFilter,
) -> Result<PaginatedResponse<Escrow>, ApiError> {
    let mut builder = sqlx::QueryBuilder::new("SELECT * FROM escrows WHERE ");

    match filter.role.as_deref() {
        Some("buyer") => {
            builder.push("buyer = ");
            builder.push_bind(user_address);
        }
        Some("farmer") => {
            builder.push("farmer = ");
            builder.push_bind(user_address);
        }
        _ => {
            builder.push("(buyer = ");
            builder.push_bind(user_address);
            builder.push(" OR farmer = ");
            builder.push_bind(user_address);
            builder.push(")");
        }
    }

    if let Some(ref status) = filter.status {
        builder.push(" AND status = ");
        builder.push_bind(status);
    }

    builder.push(" ORDER BY created_at DESC LIMIT ");
    builder.push_bind(pagination.per_page() as i64);
    builder.push(" OFFSET ");
    builder.push_bind(pagination.offset() as i64);

    let escrows = builder.build_query_as::<Escrow>().fetch_all(pool).await?;

    let mut count_builder = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM escrows WHERE ");
    match filter.role.as_deref() {
        Some("buyer") => {
            count_builder.push("buyer = ");
            count_builder.push_bind(user_address);
        }
        Some("farmer") => {
            count_builder.push("farmer = ");
            count_builder.push_bind(user_address);
        }
        _ => {
            count_builder.push("(buyer = ");
            count_builder.push_bind(user_address);
            count_builder.push(" OR farmer = ");
            count_builder.push_bind(user_address);
            count_builder.push(")");
        }
    }
    if let Some(ref status) = filter.status {
        count_builder.push(" AND status = ");
        count_builder.push_bind(status);
    }

    let total: (i64,) = count_builder.build_query_as().fetch_one(pool).await?;

    Ok(PaginatedResponse::new(
        escrows,
        total.0,
        pagination.page(),
        pagination.per_page(),
    ))
}

#[allow(dead_code)]
pub(crate) async fn get_escrow(pool: &PgPool, id: i32) -> Result<Escrow, ApiError> {
    sqlx::query_as::<_, Escrow>("SELECT * FROM escrows WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)
}

/// Fetch an escrow only when `user_address` is the buyer or farmer.
/// Returns `NotFound` for both "escrow doesn't exist" and "user is not a party"
/// so callers cannot distinguish the two cases and cannot enumerate IDs.
pub async fn get_escrow_for_user(
    pool: &PgPool,
    id: i32,
    user_address: &str,
) -> Result<Escrow, ApiError> {
    sqlx::query_as::<_, Escrow>(
        "SELECT * FROM escrows WHERE id = $1 AND (buyer = $2 OR farmer = $2)",
    )
    .bind(id)
    .bind(user_address)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)
}

pub async fn create_escrow(
    pool: &PgPool,
    buyer: &str,
    data: CreateEscrow,
) -> Result<Escrow, ApiError> {
    sqlx::query_as::<_, Escrow>(
        r#"
        INSERT INTO escrows (buyer, farmer, amount, order_id)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(buyer)
    .bind(&data.farmer)
    .bind(data.amount)
    .bind(data.order_id)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)
}

pub async fn update_escrow_status(
    pool: &PgPool,
    id: i32,
    status: EscrowStatus,
) -> Result<Escrow, ApiError> {
    sqlx::query_as::<_, Escrow>(
        "UPDATE escrows SET status = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(status)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)
}
