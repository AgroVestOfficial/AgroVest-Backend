-- Migration: Add missing database indexes for frequently queried columns
-- Issue: https://github.com/AgroVestOfficial/AgroVest-Backend/issues/7
-- Performance improvement for high-volume query patterns

-- Proposals: filter by proposer address
-- Used in: queries to get user's proposals, dashboard views
CREATE INDEX IF NOT EXISTS idx_proposals_proposer ON proposals(proposer);

-- Challenges: filter by proposal_id (JOIN with proposals)
-- Used in: proposal detail views, challenge listings per proposal
CREATE INDEX IF NOT EXISTS idx_challenges_proposal_id ON challenges(proposal_id);

-- Disputes: filter by challenge_id (JOIN with challenges) 
-- Used in: challenge detail views, dispute resolution workflows
CREATE INDEX IF NOT EXISTS idx_disputes_challenge_id ON disputes(challenge_id);

-- Votes: filter by voter address
-- Used in: user voting history, vote verification queries
CREATE INDEX IF NOT EXISTS idx_votes_voter ON votes(voter);

-- Escrows: filter by status in list queries
-- Used in: escrow status filtering, dashboard analytics
CREATE INDEX IF NOT EXISTS idx_escrows_status ON escrows(status);

-- Products: filter by sold status
-- Used in: marketplace listings, inventory management
CREATE INDEX IF NOT EXISTS idx_products_sold ON products(sold);

-- Investments: filter by owner address
-- Used in: user investment portfolios, ownership verification
CREATE INDEX IF NOT EXISTS idx_investments_owner ON investments(owner);
