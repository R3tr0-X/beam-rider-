-- 0003_rebalance_events.sql
-- Cross-chain rebalance lifecycle tracking

CREATE TABLE rebalances (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    proposed_at        TEXT NOT NULL,
    src_chain          INTEGER NOT NULL,
    dest_chain         INTEGER NOT NULL,
    amount_usdc_atoms  TEXT NOT NULL,
    venue              TEXT NOT NULL,           -- "aave-arbitrum", "moola-celo"
    expected_apy_bps   INTEGER NOT NULL,
    propose_tx         TEXT,                    -- Celo YieldStrategy.proposeStrategy
    bridge_tx          TEXT,                    -- Squid/Across or CCTP burn
    cctp_burn_tx       TEXT,
    cctp_attestation   BLOB,
    cctp_mint_tx       TEXT,                    -- destination receiveMessage
    status             TEXT NOT NULL,           -- proposed|bridging|cctp_burnt|completed|failed
    finished_at        TEXT
);
