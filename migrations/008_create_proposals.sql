CREATE TABLE IF NOT EXISTS proposals (
    id                      SERIAL PRIMARY KEY,
    proposal_id_onchain     INTEGER UNIQUE,
    title                   VARCHAR(255) NOT NULL,
    description             TEXT,
    created_at_onchain      BIGINT NOT NULL,
    ends_at                 BIGINT NOT NULL,
    required_votes          BIGINT NOT NULL,
    proposer                VARCHAR(56) NOT NULL REFERENCES users(address),
    executed                BOOLEAN NOT NULL DEFAULT false,
    is_challenged           BOOLEAN NOT NULL DEFAULT false,
    accept_votes            BIGINT NOT NULL DEFAULT 0,
    reject_votes            BIGINT NOT NULL DEFAULT 0,
    undecided_votes         BIGINT NOT NULL DEFAULT 0,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
