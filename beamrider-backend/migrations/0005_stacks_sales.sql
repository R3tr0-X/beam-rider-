-- 0005_stacks_sales.sql
-- Stacks-side payment ledger. Buyer pays STX or sBTC via signal-ledger.clar;
-- the agent verifies the print event via Hiro and inserts a row here.

CREATE TABLE stacks_sales (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     INTEGER NOT NULL REFERENCES signals(id),
    buyer         TEXT NOT NULL,
    pair          TEXT NOT NULL,
    token         TEXT NOT NULL,
    amount_atoms  TEXT NOT NULL,
    stacks_tx_id  TEXT NOT NULL UNIQUE,
    block_height  INTEGER NOT NULL,
    settled_at    TEXT NOT NULL
);
CREATE INDEX idx_stacks_sales_buyer_block
    ON stacks_sales(buyer, block_height DESC);
