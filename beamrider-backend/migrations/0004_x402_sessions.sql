-- 0004_x402_sessions.sql
-- Lump-sum x402 V2 session vouchers. Each row is one prepaid bundle
-- redeemable for `balance` signal calls until `expiry`. Decrement is
-- single-statement atomic; no Rust-side mutex needed.

CREATE TABLE sessions (
    token            TEXT PRIMARY KEY,
    buyer            TEXT NOT NULL,
    chain_id         INTEGER NOT NULL,
    paid_token       TEXT NOT NULL,
    paid_amount      TEXT NOT NULL,
    balance          INTEGER NOT NULL CHECK (balance >= 0),
    requests_used    INTEGER NOT NULL DEFAULT 0 CHECK (requests_used >= 0),
    expiry           TEXT NOT NULL,
    settle_tx_hash   TEXT NOT NULL UNIQUE,
    created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_sessions_expiry ON sessions(expiry);
CREATE INDEX idx_sessions_buyer  ON sessions(buyer);
