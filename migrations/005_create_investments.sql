CREATE TABLE IF NOT EXISTS investments (
    id                      SERIAL PRIMARY KEY,
    investment_id_onchain   INTEGER UNIQUE,
    farm_id                 INTEGER NOT NULL REFERENCES farms(id),
    image                   VARCHAR(512),
    name                    VARCHAR(255) NOT NULL,
    about                   TEXT,
    owner                   VARCHAR(56) NOT NULL REFERENCES users(address),
    min_amount              BIGINT NOT NULL,
    amount_raised           BIGINT NOT NULL DEFAULT 0,
    start_date              BIGINT NOT NULL,
    end_date                BIGINT NOT NULL,
    farm_investor_count     INTEGER NOT NULL DEFAULT 0,
    is_active               BOOLEAN NOT NULL DEFAULT true,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_investments_farm ON investments(farm_id);
CREATE INDEX IF NOT EXISTS idx_investments_active ON investments(is_active);
