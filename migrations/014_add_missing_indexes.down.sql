-- Rollback migration: Remove the indexes added in 014_add_missing_indexes.sql
-- Use this to revert the migration if needed

-- Remove indexes in reverse order
DROP INDEX IF EXISTS idx_investments_owner;
DROP INDEX IF EXISTS idx_products_sold;
DROP INDEX IF EXISTS idx_escrows_status;
DROP INDEX IF EXISTS idx_votes_voter;
DROP INDEX IF EXISTS idx_disputes_challenge_id;
DROP INDEX IF EXISTS idx_challenges_proposal_id;
DROP INDEX IF EXISTS idx_proposals_proposer;