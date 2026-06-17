use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;
use crate::utils::validators::validate_stellar_address;

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone)]
#[sqlx(type_name = "escrow_status", rename_all = "snake_case")]
pub enum EscrowStatus {
    AwaitingDelivery,
    AwaitingApproval,
    Complete,
    Dispute,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Escrow {
    pub id: i32,
    pub escrow_id_onchain: Option<i32>,
    pub buyer: String,
    pub farmer: String,
    pub amount: i64,
    pub status: EscrowStatus,
    pub order_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateEscrow {
    #[validate(custom(function = "validate_stellar_address", message = "Invalid Stellar address format"))]
    pub farmer: String,

    #[validate(range(min = 1, message = "Escrow amount must be positive"))]
    pub amount: i64,

    #[validate(range(min = 1, message = "Order ID must be positive"))]
    pub order_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct EscrowFilter {
    pub role: Option<String>,
    pub status: Option<String>,
}
