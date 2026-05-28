CREATE TABLE IF NOT EXISTS investors (
    id                      SERIAL PRIMARY KEY,
    investor_id_onchain     INTEGER UNIQUE,
    farm_id                 INTEGER NOT NULL,
    investment_id           INTEGER NOT NULL REFERENCES investments(id),
    investor_address        VARCHAR(56) NOT NULL REFERENCES users(address),
    amount                  BIGINT NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_investors_investment ON investors(investment_id);
CREATE INDEX IF NOT EXISTS idx_investors_address ON investors(investor_address);
