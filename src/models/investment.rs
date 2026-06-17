use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::utils::validators::validate_future_timestamp;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Investment {
    pub id: i32,
    pub investment_id_onchain: Option<i32>,
    pub farm_id: i32,
    pub image: Option<String>,
    pub name: String,
    pub about: Option<String>,
    pub owner: String,
    pub min_amount: i64,
    pub amount_raised: i64,
    pub start_date: i64,
    pub end_date: i64,
    pub farm_investor_count: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateInvestment {
    #[validate(range(min = 1, message = "Farm ID must be positive"))]
    pub farm_id: i32,

    #[validate(url(message = "Image must be a valid URL"))]
    pub image: Option<String>,

    #[validate(length(min = 1, max = 255, message = "Investment name must be between 1 and 255 characters"))]
    pub name: String,

    #[validate(length(max = 5000, message = "Investment description must not exceed 5000 characters"))]
    pub about: Option<String>,

    #[validate(range(min = 1, message = "Minimum investment amount must be positive"))]
    pub min_amount: i64,

    #[validate(custom(function = "validate_future_timestamp", message = "End date must be in the future"))]
    pub end_date: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn get_future_timestamp() -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        now + 86400 // 1 day in the future
    }

    fn get_past_timestamp() -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        now - 86400 // 1 day in the past (Sept 2001)
    }

    #[test]
    fn test_past_dated_investment_rejected() {
        let investment = CreateInvestment {
            farm_id: 1,
            image: Some("https://example.com/farm.jpg".to_string()),
            name: "Investment Round 1".to_string(),
            about: Some("First investment opportunity".to_string()),
            min_amount: 1000,
            end_date: get_past_timestamp(),
        };
        assert!(
            investment.validate().is_err(),
            "Scenario C: Past-dated investments must be rejected"
        );
    }

    #[test]
    fn test_expired_timestamp_constant_rejected() {
        let investment = CreateInvestment {
            farm_id: 1,
            image: Some("https://example.com/farm.jpg".to_string()),
            name: "Investment Round 1".to_string(),
            about: Some("First investment opportunity".to_string()),
            min_amount: 1000,
            end_date: 1000000001, // Sept 2001 - 25 years in the past
        };
        assert!(
            investment.validate().is_err(),
            "Scenario C: Old timestamp constant (1000000001) is 25 years in past and must be rejected"
        );
    }

    #[test]
    fn test_future_dated_investment_accepted() {
        let investment = CreateInvestment {
            farm_id: 1,
            image: Some("https://example.com/farm.jpg".to_string()),
            name: "Investment Round 1".to_string(),
            about: Some("First investment opportunity".to_string()),
            min_amount: 1000,
            end_date: get_future_timestamp(),
        };
        assert!(
            investment.validate().is_ok(),
            "Scenario C: Future-dated investments must be accepted"
        );
    }

    #[test]
    fn test_zero_minimum_amount_rejected() {
        let investment = CreateInvestment {
            farm_id: 1,
            image: Some("https://example.com/farm.jpg".to_string()),
            name: "Investment Round 1".to_string(),
            about: Some("First investment opportunity".to_string()),
            min_amount: 0,
            end_date: get_future_timestamp(),
        };
        assert!(
            investment.validate().is_err(),
            "Zero minimum investment amounts must be rejected"
        );
    }

    #[test]
    fn test_valid_investment_structure() {
        let investment = CreateInvestment {
            farm_id: 1,
            image: Some("https://example.com/farm.jpg".to_string()),
            name: "Organic Farm Investment".to_string(),
            about: Some("Invest in sustainable farming practices".to_string()),
            min_amount: 5000,
            end_date: get_future_timestamp(),
        };
        assert!(
            investment.validate().is_ok(),
            "Valid investments must pass all validation rules"
        );
    }
}
