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
    let mut where_clause = format!(
        "WHERE (buyer = '{}' OR farmer = '{}')",
        user_address, user_address
    );

    if let Some(ref role) = filter.role {
        match role.as_str() {
            "buyer" => where_clause = format!("WHERE buyer = '{}'", user_address),
            "farmer" => where_clause = format!("WHERE farmer = '{}'", user_address),
            _ => {}
        }
    }

    if let Some(ref status) = filter.status {
        where_clause.push_str(&format!(" AND status = '{}'", status));
    }

    let count_sql = format!("SELECT COUNT(*) FROM escrows {}", where_clause);
    let sql = format!(
        "SELECT * FROM escrows {} ORDER BY created_at DESC LIMIT {} OFFSET {}",
        where_clause,
        pagination.per_page(),
        pagination.offset()
    );

    let total: (i64,) = sqlx::query_as(&count_sql).fetch_one(pool).await?;
    let escrows = sqlx::query_as::<_, Escrow>(&sql).fetch_all(pool).await?;

    Ok(PaginatedResponse::new(
        escrows,
        total.0,
        pagination.page(),
        pagination.per_page(),
    ))
}

pub async fn get_escrow(pool: &PgPool, id: i32) -> Result<Escrow, ApiError> {
    sqlx::query_as::<_, Escrow>("SELECT * FROM escrows WHERE id = $1")
        .bind(id)
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
    .map_err(|e| ApiError::Database(e))
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
