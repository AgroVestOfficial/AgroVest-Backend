CREATE TABLE IF NOT EXISTS challenges (
    id                      SERIAL PRIMARY KEY,
    challenge_id_onchain    INTEGER UNIQUE,
    proposal_id             INTEGER NOT NULL REFERENCES proposals(id),
    description             TEXT,
    resolved                BOOLEAN NOT NULL DEFAULT false,
    challenger              VARCHAR(56) NOT NULL REFERENCES users(address),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS disputes (
    id                      SERIAL PRIMARY KEY,
    dispute_id_onchain      INTEGER UNIQUE,
    challenge_id            INTEGER NOT NULL REFERENCES challenges(id),
    arbitrator              VARCHAR(56) NOT NULL REFERENCES users(address),
    resolved                BOOLEAN NOT NULL DEFAULT false,
    ruling                  BOOLEAN NOT NULL DEFAULT false,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
