use crate::error::ApiError;
use crate::models::challenge::{Challenge, CreateChallenge};
use crate::models::dispute::{CreateDispute, Dispute};
use crate::models::proposal::{CreateProposal, Proposal};
use crate::models::vote::Vote;
use crate::utils::pagination::{PaginatedResponse, PaginationParams};
use sqlx::PgPool;

pub async fn list_proposals(
    pool: &PgPool,
    pagination: &PaginationParams,
) -> Result<PaginatedResponse<Proposal>, ApiError> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM proposals")
        .fetch_one(pool)
        .await?;

    let proposals = sqlx::query_as::<_, Proposal>(
        "SELECT * FROM proposals ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(pagination.per_page() as i64)
    .bind(pagination.offset() as i64)
    .fetch_all(pool)
    .await?;

    Ok(PaginatedResponse::new(
        proposals,
        total.0,
        pagination.page(),
        pagination.per_page(),
    ))
}

pub async fn get_proposal(pool: &PgPool, id: i32) -> Result<Proposal, ApiError> {
    sqlx::query_as::<_, Proposal>("SELECT * FROM proposals WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)
}

pub async fn create_proposal(
    pool: &PgPool,
    proposer: &str,
    data: CreateProposal,
) -> Result<Proposal, ApiError> {
    sqlx::query_as::<_, Proposal>(
        r#"
        INSERT INTO proposals (title, description, created_at_onchain, ends_at, required_votes, proposer)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(&data.title)
    .bind(&data.description)
    .bind(chrono::Utc::now().timestamp())
    .bind(data.ends_at)
    .bind(data.required_votes)
    .bind(proposer)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)
}

pub async fn vote_on_proposal(
    pool: &PgPool,
    voter: &str,
    proposal_id: i32,
    vote_type: &str,
) -> Result<Vote, ApiError> {
    let vt = match vote_type {
        "accept" => "accept",
        "reject" => "reject",
        _ => "undecided",
    };

    sqlx::query_as::<_, Vote>(
        r#"
        INSERT INTO votes (proposal_id, voter, voting_power, vote_type)
        VALUES ($1, $2, 1, $3::vote_type)
        ON CONFLICT (proposal_id, voter) DO UPDATE SET vote_type = $3::vote_type
        RETURNING *
        "#,
    )
    .bind(proposal_id)
    .bind(voter)
    .bind(vt)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)
}

pub async fn list_challenges(pool: &PgPool) -> Result<Vec<Challenge>, ApiError> {
    let challenges =
        sqlx::query_as::<_, Challenge>("SELECT * FROM challenges ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?;
    Ok(challenges)
}

pub async fn create_challenge(
    pool: &PgPool,
    challenger: &str,
    data: CreateChallenge,
) -> Result<Challenge, ApiError> {
    sqlx::query_as::<_, Challenge>(
        r#"
        INSERT INTO challenges (proposal_id, description, challenger)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(data.proposal_id)
    .bind(&data.description)
    .bind(challenger)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)
}

pub async fn list_disputes(pool: &PgPool) -> Result<Vec<Dispute>, ApiError> {
    let disputes = sqlx::query_as::<_, Dispute>("SELECT * FROM disputes ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(disputes)
}

pub async fn create_dispute(
    pool: &PgPool,
    disputer: &str,
    data: CreateDispute,
) -> Result<Dispute, ApiError> {
    // 1. The challenge must exist.
    let challenge = sqlx::query_as::<_, Challenge>("SELECT * FROM challenges WHERE id = $1")
        .bind(data.challenge_id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    // 2. Load the parent proposal so we can authorize the proposer as well.
    //    The FK on challenges.proposal_id guarantees this row exists.
    let proposal = sqlx::query_as::<_, Proposal>("SELECT * FROM proposals WHERE id = $1")
        .bind(challenge.proposal_id)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    // 3. Authorization: the disputer must be a party to the challenge —
    //    either the challenger who raised it or the proposer being challenged.
    if disputer != challenge.challenger && disputer != proposal.proposer {
        return Err(ApiError::Forbidden);
    }

    // 4. A resolved challenge cannot be disputed.
    if challenge.resolved {
        return Err(ApiError::BadRequest(
            "Cannot dispute a resolved challenge".to_string(),
        ));
    }

    // 5. Only one dispute is allowed per challenge.
    let existing = sqlx::query_as::<_, Dispute>("SELECT * FROM disputes WHERE challenge_id = $1")
        .bind(data.challenge_id)
        .fetch_optional(pool)
        .await?;
    if existing.is_some() {
        return Err(ApiError::Conflict(
            "A dispute already exists for this challenge".to_string(),
        ));
    }

    // 6. Create the dispute, binding the arbitrator from the authenticated
    //    user rather than the request body.
    sqlx::query_as::<_, Dispute>(
        r#"
        INSERT INTO disputes (challenge_id, arbitrator)
        VALUES ($1, $2)
        RETURNING *
        "#,
    )
    .bind(data.challenge_id)
    .bind(disputer)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)
}
