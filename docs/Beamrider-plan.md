# BeamRider — High-Fidelity Synthesis Plan

**Sources of truth.** This document fuses three inputs and is audited against the live tree at the timestamp of writing:

| Source | Bytes | Role |
|---|---|---|
| [`beamrider-brief.md`](../beamrider-brief.md) | 24 KB | Architecture v1 (POS/x402/CCTP only) |
| [`beamrider-v2.md`](../beamrider-v2.md) | 27 KB | MiniPay + Stacks Heavy + Hermes-pattern relay |
| [`beamrider-v2-ext.md`](../beamrider-v2-ext.md) | 7 KB | Two corrections (drop USDCx; full $0 deploy) |
| [`AGENTS.md`](../AGENTS.md) | 5.5 KB | Engineering invariants binding both source sub-projects |
| [`beamrider-backend/`](../beamrider-backend/) | live tree | Rust MVP already implementing brief |
| [`beamrider-contracts/`](../beamrider-contracts/) | live tree | Foundry workspace: 4 Solidity contracts already shipped |

Where v2 and the live code diverge, the live code wins and v2 is treated as a forward-looking diff. Where v2 contradicts the brief or AGENTS.md, AGENTS.md wins (it is the binding contract).

---

## 1. What changed across the three documents — a fact ledger

### 1.1 Brief (v1) → v2 — three orthogonal additions

1. **MiniPay payment path** for Celo cUSD buyers — *out-of-band of x402*, not a replacement.
2. **Stacks side** — a parallel earnings + yield loop on Stacks (STX / sBTC), four Clarity contracts, Hiro API client.
3. **Hermes-pattern relay** — Rust worker posts EVM signal hashes to a Stacks oracle, granting Stacks buyers Bitcoin-finality-anchored attestation.

Plus one cross-cutting change:

4. **x402 V2 session tokens** — replace per-request 402 challenges with lump-sum N-request vouchers.

### 1.2 v2 → v2-ext — two corrections

1. **`USDCx` is wrong**, drop it everywhere. Real Stacks yield venues are **Zest**, **Bitflow sBTC/STX LP**, **StackingDAO**.
2. **Deployment** is concrete: bare-metal Docker Compose + Cloudflare Tunnel, or Fly.io. Includes the SQLite WAL pragma (already implemented in [`db.rs:19-22`](../beamrider-backend/src/db.rs#L19-L22)) and the `read_only: true` + `tmpfs:/tmp` Docker pattern.

### 1.3 What is *already* implemented, not just specified

The MVP backend and Foundry workspace are non-empty. v2's plan items are *additions* to a live system, not greenfield. Concrete reality at audit time:

| v2 element | Live state in repo | Action required |
|---|---|---|
| `chains/stacks.rs` | absent | **NEW** |
| `workers/stacks_relay.rs` | absent | **NEW** |
| `workers/stacks_yield.rs` | absent | **NEW (scaffold only — no relayer keys yet)** |
| MiniPay verifier | trait `CeloPaymentVerifier` exists, only `Unsupported` impl ([`x402.rs:182-197`](../beamrider-backend/src/middleware/x402.rs#L182-L197)) | replace with **Forno-based real implementation** |
| x402 V2 session tokens | absent | **NEW migration + repo + middleware path** |
| `Venue` enum w/ Stacks venues | EVM-only ([`strategy.rs:4-12`](../beamrider-backend/src/domain/strategy.rs#L4-L12)) | extend |
| `migrations/0004…` and `0005…` | only `0001..0003` | **add 3 new migrations** |
| Clarity contracts | absent | **new `beamrider-stacks-contracts/` package** |
| YieldStrategy `Venue` enum | mirrors Rust EVM enum ([`YieldStrategy.sol:20-27`](../beamrider-contracts/src/YieldStrategy.sol#L20-L27)) | unchanged — Stacks is not bridgeable from this contract |
| HookData wire format | already byte-identical Rust ↔ Solidity ([`AGENTS.md:54-68`](../AGENTS.md#L54-L68)) | preserve invariant |

---

## 2. Architectural coherence — what the brief asserts, what v2 must not break

### 2.1 Binding invariants from `AGENTS.md`

These constrain everything below. Violation = revert.

- **Auth.** x402 *is* the auth layer. No parallel auth subsystem. → MiniPay and session tokens both surface inside `middleware/x402.rs` or as *peer* middleware that emits the same `VerifiedPayment`.
- **Layer purity.** Handlers thin; domain pure (no IO, no async); repositories own SQLite; services orchestrate.
- **Static dispatch by default.** Traits only at external boundaries that need test fakes (`CeloPaymentVerifier` qualifies; `SignalRepository` does not need to be a trait — the existing code uses concrete `SqliteSignalRepository`).
- **SQLite is production.** WAL, foreign keys on, busy timeout, bounded pool, indexed lookups. Already enforced by [`db.rs::connect`](../beamrider-backend/src/db.rs#L10-L31).
- **Big amounts as decimal strings.** Already enforced by `amount_atoms TEXT NOT NULL` columns and `String` typing in `VerifiedPayment`.
- **cUSD ≠ x402-USDC.** MiniPay/Forno path must not pretend to be CDP-verified.
- **Solidity hygiene.** Custom errors over revert strings, no OZ tree, immutable for ctor-time addresses, CEI without reentrancy guard when CEI is sufficient. Already followed by all four shipped contracts.

### 2.2 The single architectural risk v2 amplifies — and the mitigation

The brief flagged it: *"the CCTP V2 hooks demo physically lives on Base/Arbitrum, not Celo"*. v2 doubles the off-Celo surface (now Base + Arbitrum + Stacks + plus an off-chain relay key), so the **Celo-first story** has to be enforced architecturally, not just rhetorically:

- **Celo is the launcher.** The agent's identity is `BeamRiderRegistry` on Celo. Every other surface (Stacks oracle, hook receivers, MiniPay receipts) carries a hash bound to this Celo `tokenId`.
- **All on-chain pubkey lookups go to Celo.** The Stacks `signal-oracle.clar` stores hashes only — verification against the agent's Ed25519 pubkey is the buyer's responsibility, and the canonical pubkey lives in Celo's `BeamRiderRegistry`.
- **No separate Stacks identity.** `beamrider-registry.clar` is a *mirror* indexed by an integer `agent-id` that **must equal** the Celo `tokenId`. This is enforced by setting `agent-id = tokenId` at register time and is documented in the contract's `;;` invariant block.

### 2.3 Coherence check — does v2 contradict v1?

| Concern | v1 says | v2 says | Resolution |
|---|---|---|---|
| Yield destinations | EVM (Aave/Moola/Ubeswap) via CCTP hop | EVM **and** Stacks (Zest/Bitflow/StackingDAO) | Additive; venue picker chooses across both via `StrategyService::choose_best`. Stacks venues participate in the same `ApyQuote` ranking. |
| Auth surface | x402 only | x402 + MiniPay + Stacks-native receipts | All three converge on a uniform `VerifiedPayment` domain type. |
| Off-chain relay key | None | Authorized relayer principal on Stacks | New trust assumption; surface in README under "Honest caveats". |
| Frontend wallet | wagmi/viem only | wagmi + Stacks.js + MiniPay branch | Out of scope for *this* implementation pass (backend + contracts focus); spec-only. |

No contradictions; v2 is a strict superset. The implementation order below preserves all v1 behavior.

---

## 3. Updated end-to-end architecture

```
                 ┌────────────────────┐  ┌────────────────────┐
                 │   EVM buyers       │  │  Stacks buyers     │
                 │  (wagmi + viem)    │  │ (Leather/Xverse)   │
                 │  cUSD on Celo      │  │  STX or sBTC       │
                 │  USDC on Base/Arb  │  │  via Stacks.js     │
                 └─────────┬──────────┘  └──────────┬─────────┘
                           │                        │
                           │  402 Payment Required  │  signal-ledger.clar
                           │  + 4xx → x402 retry    │  contract-call (direct)
                           │                        │
   ┌─────────────────┐   ┌─┴────────┐   ┌───────────┴────────────┐
   │  MiniPay user   │   │ Coinbase │   │  Hiro Stacks API       │
   │  (window.eth)   │   │  CDP     │   │  (read-only)           │
   │  cUSD legacy tx │   │ x402 fac │   │  GET /extended/v1/...  │
   └────────┬────────┘   └─────┬────┘   └───────────┬────────────┘
            │                  │                    │
            ▼                  ▼                    ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │      BeamRider Rust Agent (Axum + sqlx + alloy + rig-core)      │
  │                                                                 │
  │   middleware/x402.rs  ──────────  one of three verifier modes:  │
  │     (a) CDP facilitator (USDC on Base/Arb)                      │
  │     (b) X402-Session header (lump-sum voucher → SQLite)         │
  │     (c) Forno-RPC cUSD Transfer log decode (MiniPay)            │
  │     (d) Hiro tx-event verifier (Stacks STX/sBTC sale)           │
  │                                                                 │
  │   services/signal_service.rs  → produces signed signal          │
  │   services/session_service.rs → issues / consumes vouchers      │
  │   services/strategy_service.rs → picks venue (EVM ∪ Stacks)     │
  │                                                                 │
  │   workers/stacks_relay.rs (gated by relayer key cfg)            │
  │           ↳ posts SHA-256(canonical_bytes(signal))              │
  │             to signal-oracle.clar every Stacks block            │
  └─────────────────────────────────────────────────────────────────┘
            │                                    │
            ▼                                    ▼
  ┌───────────────────────┐           ┌───────────────────────┐
  │   Celo mainnet        │           │   Stacks mainnet      │
  │  - BeamRiderRegistry  │           │  - beamrider-registry │
  │  - SignalLedger       │           │  - signal-ledger      │
  │  - YieldStrategy      │           │  - signal-oracle      │
  │     ↳ executeOnCelo   │           │  - yield-vault        │
  │     ↳ executeXChain ──┼─Squid────►│  Zest / Bitflow /     │
  │                       │           │  StackingDAO          │
  └─────────┬─────────────┘           └───────────────────────┘
            │
            │ Squid hop (Celo → Base, off-chain)
            ▼
  ┌───────────────────────────────┐
  │  Base / Arbitrum mainnet      │
  │  CCTP V2 burn-with-hook       │
  │   ↳ HookReceiver.sol          │
  │     ↳ ACTION_DEPOSIT_AAVE     │
  │     ↳ ACTION_RETURN_HOME      │
  └───────────────────────────────┘
```

---

## 4. Smart contracts — the full picture

### 4.1 Solidity (Celo + Base + Arbitrum) — *unchanged from v1, all four shipped*

- [`BeamRiderRegistry.sol`](../beamrider-contracts/src/BeamRiderRegistry.sol) — agent identity. Each agent is a `tokenId`. Two-step ownership transfer.
- [`SignalLedger.sol`](../beamrider-contracts/src/SignalLedger.sol) — `recordSale(saleId, agentTokenId, signalHash, feeToken, paidAmount)`. Allow-listed fee tokens. Replay-protected on `saleId`. CEI.
- [`YieldStrategy.sol`](../beamrider-contracts/src/YieldStrategy.sol) — `proposeStrategy → execute{OnCelo,CrossChain}`. Custody-only on the cross-chain path; bridge router pulls funds. Custom errors throughout. **Preserve unchanged.**
- [`HookReceiver.sol`](../beamrider-contracts/src/HookReceiver.sol) — `IMessageHandlerV2` implementor on Base / Arbitrum. Three guards: transmitter / source domain / sender. Unfinalized rejected. **Preserve unchanged.**
- [`HookDataCodec.sol`](../beamrider-contracts/src/libraries/HookDataCodec.sol) ↔ [`cctp.rs::encode_hook_data`](../beamrider-backend/src/chains/cctp.rs#L39-L51) — wire-format invariant. **Do not touch without updating both.**

### 4.2 Clarity (Stacks) — **new package `beamrider-stacks-contracts/`**

Mirrors the Solidity surface; written in Clarity 4 syntax. Test scaffold via Clarinet.

```
beamrider-stacks-contracts/
├── Clarinet.toml
├── settings/
│   └── Devnet.toml
├── contracts/
│   ├── beamrider-registry.clar    # agent identity (mirrors Celo tokenId)
│   ├── signal-ledger.clar         # STX / sBTC payment receipts
│   ├── signal-oracle.clar         # Hermes-pattern signal hash commitments
│   └── yield-vault.clar           # Zest / Bitflow / StackingDAO routing
└── tests/                         # clarinet integrate / vitest
```

#### `beamrider-registry.clar`

| Concern | Decision |
|---|---|
| ID space | **Caller supplies `agent-id` (= Celo tokenId)**; first registrant wins. Avoids drift between chains. |
| Pubkey | `(buff 32)` Ed25519. Same key as Celo. |
| Mutability | `update-metadata` and `transfer-ownership` mirror the Solidity registry (single-step on Stacks; two-step is overkill given on-chain accountability). |
| Errors | `(err u404)` unknown, `(err u403)` not owner, `(err u400)` invalid input. |

#### `signal-ledger.clar`

| Concern | Decision |
|---|---|
| Receivers | `(define-data-var agent-receiver principal …)` — owner-mutable. |
| Tokens | STX (native `stx-transfer?`) and sBTC (SIP-010 `transfer`). Hard-pinned to a configured sBTC contract principal (no allow-list — only one canonical sBTC token). |
| Replay | `(define-map sales { sale-id: (buff 32) } { recorded-at: uint })` rejects repeats. |
| Event | `print` block emits `{ event: "signal-sale", buyer, agent-id, pair, token, amount, sale-id, block-height }`. |

#### `signal-oracle.clar`

| Concern | Decision |
|---|---|
| Auth | `(define-data-var authorized-relayer principal …)` — owner-mutable; only the relayer can call `commit-signal`. |
| State | `{ pair: (string-utf8 20), block-height: uint } → { hash: (buff 32), confidence-bps: uint }`. |
| Read | `get-signal` returns the latest commitment for a pair (last-write-wins per block). |

#### `yield-vault.clar`

| Concern | Decision |
|---|---|
| Auth | `vault-owner` only — no public deposits; the Rust agent calls. |
| Venues | Three explicit functions: `deposit-zest`, `deposit-bitflow-lp`, `deposit-stacking-dao`. Each takes `(amount uint)` and a token principal. **No generic arbitrary-call function** — that's an unbounded blast radius. |
| Token contract | Pinned at deploy to a sBTC SIP-010 trait reference. |
| Events | Each deposit emits a typed `print` block for off-chain indexing. |

### 4.3 Why no new Solidity contracts

v2 introduces no requirement that adds a new Solidity surface. Specifically:

- **Sessions are off-chain.** `sessions` table in SQLite is sufficient; on-chain session voucher would burn gas every issue/consume. Reject. (If POS needs more Celo tx counts, the existing `SignalLedger.recordSale` already serves.)
- **MiniPay is just a cUSD ERC-20 transfer.** No new contract; the Forno verifier reads the existing cUSD contract logs.
- **Stacks doesn't bridge to Celo via CCTP.** The Squid/Across hop and the CCTP V2 hop are unchanged from v1. `YieldStrategy.executeStrategyCrossChain` already supports this by being bridge-agnostic (transfers custody to a configured router).

Adding a new Solidity contract here would be **bloat without function**.

---

## 5. Backend — the precise diff against the live tree

### 5.1 New modules

```
beamrider-backend/src/
├── chains/
│   └── stacks.rs                    # NEW: Hiro read-only client
├── middleware/
│   └── x402.rs                      # AMENDED: session token path
├── services/
│   ├── celo_payment.rs              # NEW: Forno cUSD verifier (MiniPay)
│   └── session_service.rs           # NEW: voucher issue / consume
├── repositories/
│   ├── session_repo.rs              # NEW: sessions table CRUD
│   ├── stacks_sale_repo.rs          # NEW: stacks_sales insert + count
│   └── minipay_repo.rs              # NEW: minipay_payments insert (replay)
├── handlers/
│   ├── sessions.rs                  # NEW: POST /v1/sessions
│   └── signals.rs                   # AMENDED: 3-path verification
├── workers/
│   └── stacks_relay.rs              # NEW: scaffold (gated by relayer cfg)
├── domain/
│   ├── strategy.rs                  # AMENDED: Stacks venues
│   └── stacks.rs                    # NEW: StacksTxRef, StacksSale
└── dto/
    ├── x402.rs                      # AMENDED: SessionRequest, SessionToken
    └── minipay.rs                   # NEW: MiniPayHeader
```

### 5.2 Migration plan — three new migrations, sequenced after `0003`

```
migrations/
├── 0004_x402_sessions.sql          # NEW
├── 0005_stacks_sales.sql           # NEW
└── 0006_minipay_payments.sql       # NEW
```

Schemas codified in §6.

### 5.3 Verifier modes — the union of three payment surfaces

`X402Verifier` already discriminates `Cdp` and `Fixture`. Extension:

```text
enum X402VerifierMode {
    Cdp,       // existing: USDC on Base/Arbitrum via Coinbase facilitator
    Fixture,   // existing: test-only header
    Session,   // NEW: X402-Session: <token>
    Celo,      // NEW: X-MiniPay-TxHash: <0x…> via Forno
    Stacks,    // NEW: X-Stacks-TxId: <0x…> via Hiro
}
```

Each mode produces the **same `VerifiedPayment`** domain type, so downstream code (the signal service, sale repository) stays uniform.

The verifier *picks a mode by header presence*, in this order: `X402-Session` → `X-MiniPay-TxHash` → `X-Stacks-TxId` → `x-payment` (CDP/fixture) → 402.

### 5.4 SOLID, anti-bloat, anti-perf-trap discipline

- **SRP** — each new repo owns exactly one table; each new service one orchestration concern (issue voucher, verify Celo tx, verify Stacks tx).
- **OCP** — verifier dispatch is a `match` on a header-derived mode, *not* a new trait per backend. Adding a future mode = one variant + one impl block.
- **LSP** — `CeloPaymentVerifier` trait stays (only because `Unsupported` and `Forno` need to be swapped at build time without runtime cost). `StacksPaymentVerifier` is **concrete** — there is exactly one production impl.
- **ISP** — Hiro client exposes `verify_signal_sale`, `get_balance`. It does *not* expose tx broadcast (no signing key needed; we don't trust ourselves to write that without a real test rig).
- **DIP** — services depend on the concrete Hiro `StacksClient`, *not* on a fake. Tests use a stub HTTP server (`mockito`) — same pattern as the rest of the codebase.

Performance traps actively avoided:

- ❌ No `dyn Trait` on the request hot path. Verifier dispatch stays static via `match`.
- ❌ No per-request `reqwest::Client::new()` — reuse the cloneable client already in `AppState`.
- ❌ No SQLite full-table scans — every lookup is by primary key or covered index.
- ❌ No mutex around the SQLite pool — sqlx's pool is already concurrency-safe.
- ❌ No global `Lazy<…>` registry of verifiers — they are owned by `AppState`.
- ❌ Voucher decrement is **single-statement atomic** (`UPDATE … SET balance = balance - 1 WHERE token = ? AND balance > 0 AND expiry > datetime('now')`), no Rust-side mutex.

### 5.5 SQLite usage patterns (preserved from AGENTS.md, sharpened)

- **WAL + NORMAL synchronous** — already in [`db.rs:19-22`](../beamrider-backend/src/db.rs#L19-L22).
- **Foreign keys ON** — already.
- **`busy_timeout(5s)`** — already.
- **Bounded pool** — `sqlite_max_connections` configurable, default 5.
- **Indexes on hot lookups**:
  - `sessions(token)` is PRIMARY KEY.
  - `sessions(expiry)` for cleanup sweep.
  - `stacks_sales(stacks_tx_id)` UNIQUE — replay protection.
  - `stacks_sales(buyer, block_height)` for buyer queries.
  - `minipay_payments(tx_hash)` UNIQUE — replay protection.
- **Decimal-string amounts** for STX/sBTC/cUSD/USDC.
- **All multi-row writes inside a single transaction** — already done in [`sale_repo.rs:21-67`](../beamrider-backend/src/repositories/sale_repo.rs#L21-L67).

---

## 6. SQLite migrations — exact text (to be written)

### 6.1 `0004_x402_sessions.sql`

```sql
-- 0004_x402_sessions.sql
-- Lump-sum vouchers (x402 V2 sessions). One row = one prepaid bundle.
-- Concurrency-safe via single-statement atomic decrement.

CREATE TABLE sessions (
    token            TEXT PRIMARY KEY,         -- base64url(32 random bytes)
    buyer            TEXT NOT NULL,            -- 0x… lowercase or SP… for Stacks
    chain_id         INTEGER NOT NULL,         -- EIP-155 id; -1 reserved for Stacks
    paid_token       TEXT NOT NULL,            -- ERC-20 / SIP-010 contract address
    paid_amount      TEXT NOT NULL,            -- decimal-string atoms
    balance          INTEGER NOT NULL,         -- requests remaining (>= 0)
    requests_used    INTEGER NOT NULL DEFAULT 0,
    expiry           TEXT NOT NULL,            -- RFC3339 UTC
    settle_tx_hash   TEXT NOT NULL UNIQUE,     -- on-chain settlement; replay-protected
    created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_sessions_expiry ON sessions(expiry);
CREATE INDEX idx_sessions_buyer  ON sessions(buyer);
```

### 6.2 `0005_stacks_sales.sql`

```sql
-- 0005_stacks_sales.sql
-- Stacks-side payment ledger. Mirrors `sales` for the Stacks payment surface.

CREATE TABLE stacks_sales (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     INTEGER NOT NULL REFERENCES signals(id),
    buyer         TEXT NOT NULL,                -- Stacks principal SP… / ST…
    pair          TEXT NOT NULL,                -- "ETH-USD" etc. — uppercase
    token         TEXT NOT NULL,                -- "stx" or "sbtc"
    amount_atoms  TEXT NOT NULL,                -- µSTX or sats; decimal string
    stacks_tx_id  TEXT NOT NULL UNIQUE,         -- 0x… 32 bytes hex
    block_height  INTEGER NOT NULL,
    settled_at    TEXT NOT NULL                 -- RFC3339 UTC
);
CREATE INDEX idx_stacks_sales_buyer_block
    ON stacks_sales(buyer, block_height DESC);
```

### 6.3 `0006_minipay_payments.sql`

```sql
-- 0006_minipay_payments.sql
-- MiniPay (Celo cUSD) on-chain payment ledger.
-- Verified via Forno JSON-RPC; not part of x402 / CDP.

CREATE TABLE minipay_payments (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id     INTEGER REFERENCES signals(id),
    buyer         TEXT NOT NULL,                -- 0x… lowercase
    pair          TEXT NOT NULL,
    amount_atoms  TEXT NOT NULL,                -- 1e18 atoms; decimal string
    token         TEXT NOT NULL,                -- 0x765D… cUSD
    tx_hash       TEXT NOT NULL UNIQUE,         -- replay protection
    block_number  INTEGER NOT NULL,
    settled_at    TEXT NOT NULL
);
CREATE INDEX idx_minipay_buyer ON minipay_payments(buyer);
```

---

## 7. Trust boundaries — explicit list

| Boundary | Trusted to do what | Not trusted to do what |
|---|---|---|
| `BeamRiderRegistry` (Celo) | source of truth for `tokenId → pubkey` | nothing else |
| Forno (Celo public RPC) | return real receipts; eventual consistency tolerated | be available 100% — fallback to Alchemy free tier |
| Hiro (Stacks public API) | return real tx + balance reads | do not blindly trust per-tx confirmations — require N block depth |
| Coinbase CDP facilitator | verify x402 EIP-3009 payment for USDC on Base/Arbitrum | verify cUSD on Celo (architecturally not supported) |
| `signal-oracle.clar` (Stacks) | record the relayer's commits | the relayer key itself is custodial — own that risk in the README |
| MiniPay-injected provider | identify itself via `isMiniPay`; submit cUSD txes | sign x402 payloads (no EIP-3009 support) |

---

## 8. New API contract — additions only

| Method + Path | Auth header | Body | Response |
|---|---|---|---|
| `GET /v1/signals/:pair` | one of: `x-payment` (CDP), `X402-Session`, `X-MiniPay-TxHash`, `X-Stacks-TxId` | none | `200 SignalResponse` |
| `POST /v1/sessions` | `x-payment` (CDP-verified one-shot) | `{ "requests": 20 }` | `200 { token, balance, expiry }` |

`POST /v1/sessions` is intentionally x402-only at issue time — it converts a single x402 payment into N free signal calls. Keeps the credit primitive portable.

---

## 9. Workers — disabled-by-default discipline

The repo's existing pattern ([`workers/mod.rs:9-19`](../beamrider-backend/src/workers/mod.rs#L9-L19)) gates worker spawn on `enable_workers`. Apply the same to `stacks_relay`:

- `STACKS_RELAYER_PRIVATE_KEY` and `STACKS_ORACLE_CONTRACT` env → both required to actually post; absent → log one info line and exit.
- `STACKS_RELAY_ENABLED` env → master kill-switch independent of `enable_workers`.

**No live broadcast code in this pass.** The scaffold logs intent and exits — exactly like [`earnings_watcher.rs:5-9`](../beamrider-backend/src/workers/earnings_watcher.rs#L5-L9). Reason: signing Stacks transactions safely requires test rigs we do not have. Per AGENTS.md: *"Avoid speculative abstractions, unused workers, or live chain transaction code before credentials and tests exist."*

---

## 10. Build sequence (revised, additive over the brief's 4-week)

| Week | v1 (brief) deliverables (preserved) | v2 additions (this pass and onward) |
|---|---|---|
| 1 | Registry + SignalLedger on Celo mainnet; bare Rust agent; one paid endpoint | Clarity registry + signal-ledger on Stacks mainnet (`clarinet contract publish`); new migrations 0004-0006; Stacks venue enum |
| 2 | HookReceiver on Base/Arb; rebalance service Celo→Base via Squid; YieldStrategy on Celo | signal-oracle + yield-vault on Stacks; **disabled** stacks_relay scaffold; MiniPay Forno verifier; x402 V2 session endpoint |
| 3 | Frontend dashboard; demo loop | Frontend Stacks.js + MiniPay branch (out of this pass) |
| 4 | Polish + 2nd milestone | Bitflow LP wiring; demo with three payment paths |

This pass implements all of week-1 v2 additions and the verifier/session backend changes from week 2. The Stacks relay broadcast and the frontend remain spec-only.

---

## 11. Open risks — call them out, do not hide them

1. **Stacks Rust signing.** `stacks-transactions-rs` is community-maintained. We don't broadcast in this pass; deferred until the relay key has a test rig. Mitigation: read-only client now; broadcast in a follow-up gated by config.
2. **Forno rate limits.** Cache `eth_chainId`, `eth_blockNumber` if MiniPay traffic ramps. Not done in this pass — fall through to a 429 + retry on the buyer side.
3. **Clarinet not installed locally.** Clarity contracts ship as source; `clarinet check` and Vitest tests will be run on a host that has Clarinet. Documented as such.
4. **Two on-chain identities.** Stacks `beamrider-registry.clar` mirrors Celo's `tokenId`. If a holder transfers Celo ownership but not Stacks ownership (or vice-versa), they desync. Mitigation: documentation now; an indexer that flags drift is a v2.5 item.
5. **MiniPay finality.** Celo finality on the L1 is ~5 s post-Halmos. We require `block_number ≤ latest - confirmations` with `confirmations` configurable (default 1). Tradeoff: faster UX vs. reorg risk on a low-stake $0.10 sale.
6. **Gemini free-tier rate.** Brief already documents 15 req/min, 1M token/day. Sessions help here: a 20-request voucher costs the buyer one CDP call and the agent one LLM call worth of cache cost (signals are cached 60 s per pair).

---

## 12. Testing strategy

| Layer | Tool | Scope |
|---|---|---|
| Rust unit + integration | `cargo test` | Domain pair normalization, session decrement atomicity, MiniPay receipt decoding (mocked Forno), Stacks tx parse (mocked Hiro), x402 verifier mode dispatch |
| Solidity | `forge test` | Unchanged — existing four contracts |
| Clarity | `clarinet check` (offline, no deploy) and Vitest sim tests | Run on a host with Clarinet installed; not gated in CI yet |
| End-to-end | manual demo | The three payment paths converge into the same `SignalResponse` |

Determinism: tests must pass with no env vars (`AGENTS.md:95`). Implies the Forno verifier and Hiro client are constructed lazily / from injected config; `for_test()` produces the existing fixture-x402 mode.

---

## 13. Numbered topic index

For reference and review correlation:

1. Fact ledger across the three documents
2. Architectural coherence + binding invariants
3. End-to-end architecture diagram
4. Smart contracts (Solidity preserved + Clarity new)
5. Backend diff against live tree
6. Migration text
7. Trust boundaries
8. New API contract
9. Worker discipline
10. Build sequence
11. Open risks
12. Testing strategy
13. (this section)

---

*Generated by synthesizing v1 brief + v2 + v2-ext against the live tree at 2026-05-09. The plan is binding for the implementation pass that follows; deltas are reported in the Final Report attached to the same change set.*
