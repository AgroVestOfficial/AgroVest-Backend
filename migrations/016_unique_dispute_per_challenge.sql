-- Enforce one dispute per challenge at the database level. The service layer in
-- dao_service::create_dispute already returns 409 Conflict when a dispute exists
-- for a challenge, but that check races under concurrent requests. This unique
-- index is the authoritative guarantee backing the business rule (issue #5).
--
-- A unique index is used instead of ADD CONSTRAINT so the migration is
-- idempotent via IF NOT EXISTS, consistent with the project convention.
CREATE UNIQUE INDEX IF NOT EXISTS uq_disputes_challenge_id
    ON disputes (challenge_id);
