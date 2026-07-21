mod helpers;

use sqlx::PgPool;

/// Test that upsert_user returns a user when called twice with the same address.
#[tokio::test]
#[ignore = "requires live Postgres test database"]
async fn upsert_same_address_twice_returns_user_both_times() {
    let pool = test_pool().await;
    let address = "GAHJJJKMOKYE4RVPZEWZTKH5FVI4PA3VL7GK2LFNUBSGBV7REEX6XCLD";

    // First upsert should insert a new user.
    let user1 = agrovest_backend::services::user_service::upsert_user(&pool, address)
        .await
        .expect("first upsert should succeed");
    assert_eq!(user1.address, address);

    // Second upsert should return the existing user.
    let user2 = agrovest_backend::services::user_service::upsert_user(&pool, address)
        .await
        .expect("second upsert should succeed");
    assert_eq!(user2.address, address);
}

async fn test_pool() -> PgPool {
    use sqlx::postgres::PgPoolOptions;
    use std::env;
    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/agrovest_test".to_owned());
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await
        .expect("test DB connection failed")
}
