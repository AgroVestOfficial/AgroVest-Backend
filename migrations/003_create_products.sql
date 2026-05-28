CREATE TABLE IF NOT EXISTS products (
    id                  SERIAL PRIMARY KEY,
    product_id_onchain  INTEGER UNIQUE,
    product_name        VARCHAR(255) NOT NULL,
    product_image       VARCHAR(512),
    product_description TEXT,
    product_price       BIGINT NOT NULL,
    product_owner       VARCHAR(56) NOT NULL REFERENCES users(address),
    farm_id             INTEGER REFERENCES farms(id),
    sold                BOOLEAN NOT NULL DEFAULT false,
    category            VARCHAR(100),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_products_owner ON products(product_owner);
CREATE INDEX IF NOT EXISTS idx_products_farm ON products(farm_id);
CREATE INDEX IF NOT EXISTS idx_products_category ON products(category);
