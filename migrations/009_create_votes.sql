CREATE TYPE vote_type AS ENUM ('null', 'accept', 'reject', 'undecided');

CREATE TABLE IF NOT EXISTS votes (
    id              SERIAL PRIMARY KEY,
    proposal_id     INTEGER NOT NULL REFERENCES proposals(id),
    voter           VARCHAR(56) NOT NULL REFERENCES users(address),
    voting_power    BIGINT NOT NULL,
    vote_type       vote_type NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(proposal_id, voter)
);
