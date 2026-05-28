CREATE TABLE IF NOT EXISTS ipfs_metadata (
    cid             VARCHAR(255) PRIMARY KEY,
    original_name   VARCHAR(255),
    mime_type       VARCHAR(100),
    size_bytes      BIGINT,
    uploader        VARCHAR(56) REFERENCES users(address),
    pinned          BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
