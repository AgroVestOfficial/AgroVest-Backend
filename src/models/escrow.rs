use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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

#[derive(Debug, Deserialize)]
pub struct CreateEscrow {
    pub farmer: String,
    pub amount: i64,
    pub order_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct EscrowFilter {
    pub role: Option<String>,
    pub status: Option<String>,
}
