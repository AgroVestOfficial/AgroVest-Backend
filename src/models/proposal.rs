use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::utils::validators::validate_future_timestamp;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Proposal {
    pub id: i32,
    pub proposal_id_onchain: Option<i32>,
    pub title: String,
    pub description: Option<String>,
    pub created_at_onchain: i64,
    pub ends_at: i64,
    pub required_votes: i64,
    pub proposer: String,
    pub executed: bool,
    pub is_challenged: bool,
    pub accept_votes: i64,
    pub reject_votes: i64,
    pub undecided_votes: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProposal {
    #[validate(length(min = 1, max = 500, message = "Proposal title must be between 1 and 500 characters"))]
    pub title: String,

    #[validate(length(max = 5000, message = "Proposal description must not exceed 5000 characters"))]
    pub description: Option<String>,

    #[validate(range(min = 1, message = "Required votes must be at least 1 to ensure valid quorum"))]
    pub required_votes: i64,

    #[validate(custom(function = "validate_future_timestamp", message = "Proposal end date must be in the future"))]
    pub ends_at: i64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct VoteRequest {
    #[validate(length(min = 1, max = 50, message = "Vote must be between 1 and 50 characters"))]
    pub vote: String,
}

#[derive(Debug, Deserialize, Validate)]
#[allow(dead_code)]
pub struct ExecuteProposal {
    #[validate(range(min = 1, message = "Farm ID must be positive"))]
    pub farm_id: i32,

    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: String,

    #[validate(length(max = 5000, message = "Description must not exceed 5000 characters"))]
    pub about: Option<String>,

    #[validate(range(min = 1, message = "Minimum amount must be positive"))]
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
        now - 86400 // 1 day in the past
    }

    #[test]
    fn test_zero_votes_rejected() {
        let proposal = CreateProposal {
            title: "Test Proposal".to_string(),
            description: Some("A test proposal".to_string()),
            required_votes: 0,
            ends_at: get_future_timestamp(),
        };
        assert!(
            proposal.validate().is_err(),
            "Scenario B: Zero-vote proposals must be rejected"
        );
    }

    #[test]
    fn test_positive_votes_accepted() {
        let proposal = CreateProposal {
            title: "Test Proposal".to_string(),
            description: Some("A test proposal".to_string()),
            required_votes: 1,
            ends_at: get_future_timestamp(),
        };
        assert!(
            proposal.validate().is_ok(),
            "Scenario B: Positive required votes must be accepted"
        );
    }

    #[test]
    fn test_past_dated_proposal_rejected() {
        let proposal = CreateProposal {
            title: "Test Proposal".to_string(),
            description: Some("A test proposal".to_string()),
            required_votes: 1,
            ends_at: get_past_timestamp(),
        };
        assert!(
            proposal.validate().is_err(),
            "Scenario C: Past-dated proposals must be rejected (ends_at must be in future)"
        );
    }

    #[test]
    fn test_future_dated_proposal_accepted() {
        let proposal = CreateProposal {
            title: "Test Proposal".to_string(),
            description: Some("A test proposal".to_string()),
            required_votes: 1,
            ends_at: get_future_timestamp(),
        };
        assert!(
            proposal.validate().is_ok(),
            "Scenario C: Future-dated proposals must be accepted"
        );
    }

    #[test]
    fn test_valid_proposal_structure() {
        let proposal = CreateProposal {
            title: "Approve New Investment Round".to_string(),
            description: Some("This proposal approves a new investment opportunity".to_string()),
            required_votes: 10,
            ends_at: get_future_timestamp(),
        };
        assert!(
            proposal.validate().is_ok(),
            "Valid proposals must pass all validation rules"
        );
    }
}
