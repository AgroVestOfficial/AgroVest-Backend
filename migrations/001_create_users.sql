CREATE TABLE IF NOT EXISTS users (
    address         VARCHAR(56) PRIMARY KEY,
    display_name    VARCHAR(255),
    bio             TEXT,
    avatar_url      VARCHAR(512),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
