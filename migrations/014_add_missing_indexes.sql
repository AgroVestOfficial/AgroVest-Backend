-- Migration: Add missing database indexes for frequently queried columns
-- Issue: https://github.com/AgroVestOfficial/AgroVest-Backend/issues/7
-- Performance improvement for high-volume query patterns

-- IMPORTANT: CONCURRENTLY cannot run inside a transaction.
-- sqlx runs migrations inside a transaction by default.
-- To apply this migration in production with zero downtime, run manually:
--
-- psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_proposals_proposer ON proposals(proposer);"
-- psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_challenges_proposal_id ON challenges(proposal_id);"
-- psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_disputes_challenge_id ON disputes(challenge_id);"
-- psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_votes_voter ON votes(voter);"
-- psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_escrows_status ON escrows(status);"
-- psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_products_sold ON products(sold);"
-- psql $DATABASE_URL -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_investments_owner ON investments(owner);"
--
-- For development/staging (with downtime acceptable), use the statements below:

-- Proposals: filter by proposer address
-- Used in: queries to get user's proposals, dashboard views
-- Write impact: LOW (proposer is set once on insert, never updated)
CREATE INDEX IF NOT EXISTS idx_proposals_proposer ON proposals(proposer);

-- Challenges: filter by proposal_id (JOIN with proposals)
-- Used in: proposal detail views, challenge listings per proposal
-- Write impact: LOW (proposal_id is set once on insert, never updated)
CREATE INDEX IF NOT EXISTS idx_challenges_proposal_id ON challenges(proposal_id);

-- Disputes: filter by challenge_id (JOIN with challenges) 
-- Used in: challenge detail views, dispute resolution workflows
-- Write impact: LOW (challenge_id is set once on insert, never updated)
CREATE INDEX IF NOT EXISTS idx_disputes_challenge_id ON disputes(challenge_id);

-- Votes: filter by voter address
-- Used in: user voting history, vote verification queries
-- Write impact: LOW (voter is set once on insert, never updated)
CREATE INDEX IF NOT EXISTS idx_votes_voter ON votes(voter);

-- Escrows: filter by status in list queries
-- Used in: escrow status filtering, dashboard analytics
-- Write impact: MEDIUM (status updated during escrow lifecycle, acceptable overhead)
CREATE INDEX IF NOT EXISTS idx_escrows_status ON escrows(status);

-- Products: filter by sold status
-- Used in: marketplace listings, inventory management
-- Write impact: MEDIUM (sold updated when product is purchased, acceptable overhead)
CREATE INDEX IF NOT EXISTS idx_products_sold ON products(sold);

-- Investments: filter by owner address
-- Used in: user investment portfolios, ownership verification
-- Write impact: LOW (owner is set once on insert, rarely transferred)
CREATE INDEX IF NOT EXISTS idx_investments_owner ON investments(owner);
