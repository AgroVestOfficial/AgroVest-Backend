CREATE TABLE IF NOT EXISTS reviews (
    id              SERIAL PRIMARY KEY,
    reviewer        VARCHAR(56) NOT NULL REFERENCES users(address),
    review_text     TEXT NOT NULL,
    product_id      INTEGER NOT NULL REFERENCES products(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(reviewer, product_id)
);

CREATE INDEX IF NOT EXISTS idx_reviews_product ON reviews(product_id);
