-- 0002_signal_sales.sql
-- Signal records and x402 sale ledger

CREATE TABLE signals (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    pair         TEXT NOT NULL,                 -- "ETH-USD"
    kind         INTEGER NOT NULL,              -- enum: BUY=0,SELL=1,HOLD=2
    value_bps    INTEGER NOT NULL,              -- e.g. confidence in bps
    confidence   REAL NOT NULL,                 -- 0..1
    created_at   TEXT NOT NULL,
    signature    BLOB NOT NULL                  -- Ed25519 64 bytes
);
CREATE INDEX idx_signals_pair_created ON signals(pair, created_at DESC);

CREATE TABLE sales (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     INTEGER NOT NULL REFERENCES signals(id),
    buyer         TEXT NOT NULL,
    chain_id      INTEGER NOT NULL,             -- 42220 Celo, 8453 Base, etc.
    token         TEXT NOT NULL,                -- USDC/cUSD address
    amount_atoms  TEXT NOT NULL,                -- string for 1e18 safety
    tx_hash       TEXT NOT NULL UNIQUE,
    settled_at    TEXT NOT NULL
);
