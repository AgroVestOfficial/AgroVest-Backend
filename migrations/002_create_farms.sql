CREATE TABLE IF NOT EXISTS farms (
    id                  SERIAL PRIMARY KEY,
    farm_id_onchain     INTEGER UNIQUE,
    business_name       VARCHAR(255) NOT NULL,
    business_image      VARCHAR(512),
    business_location   VARCHAR(512),
    business_contact    VARCHAR(255),
    business_email      VARCHAR(255),
    farmer_address      VARCHAR(56) NOT NULL REFERENCES users(address),
    is_registered       BOOLEAN NOT NULL DEFAULT true,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_farms_farmer_address ON farms(farmer_address);
