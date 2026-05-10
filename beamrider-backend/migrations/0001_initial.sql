-- 0001_initial.sql
-- Agent identity records, synced from on-chain BeamRiderRegistry

CREATE TABLE agents (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    on_chain_id INTEGER NOT NULL UNIQUE,        -- BeamRiderRegistry tokenId
    owner       TEXT NOT NULL,                  -- 0x... lowercase
    pubkey      BLOB NOT NULL,                  -- Ed25519 32 bytes
    name        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
