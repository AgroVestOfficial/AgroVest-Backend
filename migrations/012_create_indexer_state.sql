CREATE TABLE IF NOT EXISTS indexer_state (
    contract_address    VARCHAR(56) PRIMARY KEY,
    last_cursor         VARCHAR(255),
    last_synced_at      TIMESTAMPTZ,
    synced_height       BIGINT
);
