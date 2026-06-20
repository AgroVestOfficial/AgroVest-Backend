use sqlx::PgPool;

pub struct EscrowFixture {
    pub id: i32,
    #[allow(dead_code)]
    pub buyer: String,
    #[allow(dead_code)]
    pub farmer: String,
    pub amount: i64,
}

/// Insert a minimal escrow row and return its id and party addresses.
/// Requires the `orders` table to have a row with `order_id` first, so we
/// insert a dummy order inline.
pub async fn seed_escrow(pool: &PgPool, buyer: &str, farmer: &str) -> EscrowFixture {
    let order_id: i32 = sqlx::query_scalar(
        "INSERT INTO orders (buyer, total_price, status) VALUES ($1, 1000, 'pending') RETURNING id",
    )
    .bind(buyer)
    .fetch_one(pool)
    .await
    .expect("seed order failed");

    let id: i32 = sqlx::query_scalar(
        "INSERT INTO escrows (buyer, farmer, amount, order_id) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(buyer)
    .bind(farmer)
    .bind(500_i64)
    .bind(order_id)
    .fetch_one(pool)
    .await
    .expect("seed escrow failed");

    EscrowFixture {
        id,
        buyer: buyer.to_owned(),
        farmer: farmer.to_owned(),
        amount: 500,
    }
}
