CREATE TYPE escrow_status AS ENUM ('awaiting_delivery', 'awaiting_approval', 'complete', 'dispute');

CREATE TABLE IF NOT EXISTS escrows (
    id                  SERIAL PRIMARY KEY,
    escrow_id_onchain   INTEGER UNIQUE,
    buyer               VARCHAR(56) NOT NULL REFERENCES users(address),
    farmer              VARCHAR(56) NOT NULL REFERENCES users(address),
    amount              BIGINT NOT NULL,
    status              escrow_status NOT NULL DEFAULT 'awaiting_delivery',
    order_id            INTEGER NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_escrows_buyer ON escrows(buyer);
CREATE INDEX IF NOT EXISTS idx_escrows_farmer ON escrows(farmer);
