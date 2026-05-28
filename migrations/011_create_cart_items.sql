CREATE TABLE IF NOT EXISTS cart_items (
    id              SERIAL PRIMARY KEY,
    user_address    VARCHAR(56) NOT NULL REFERENCES users(address),
    product_id      INTEGER NOT NULL REFERENCES products(id),
    added_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_address, product_id)
);

CREATE INDEX IF NOT EXISTS idx_cart_user ON cart_items(user_address);
