-- 0006_minipay_payments.sql
-- MiniPay (Celo cUSD) on-chain payment ledger.
-- Verified via Forno JSON-RPC by decoding the cUSD ERC-20 Transfer log.
-- Distinct from x402/CDP and from the Stacks ledger.

CREATE TABLE minipay_payments (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     INTEGER REFERENCES signals(id),
    buyer         TEXT NOT NULL,
    pair          TEXT NOT NULL,
    amount_atoms  TEXT NOT NULL,
    token         TEXT NOT NULL,
    tx_hash       TEXT NOT NULL UNIQUE,
    block_number  INTEGER NOT NULL,
    settled_at    TEXT NOT NULL
);
CREATE INDEX idx_minipay_buyer ON minipay_payments(buyer);
