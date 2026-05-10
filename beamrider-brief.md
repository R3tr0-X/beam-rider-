# BeamRider — Implementation Brief

An autonomous agent on Celo that sells price/market signals to other agents over x402, accepts payment in cUSD/USDC, and routes earnings cross-chain via CCTP V2 hooks to wherever yield is best.

The name is signal-processing: a *beamrider* missile homes on a guidance beam projected by its launcher; here, buyer agents ride BeamRider's signal beam to make their own decisions, while BeamRider itself rides the CCTP beam to redeploy capital.

---

## 1. Project framing for POS submission

**Tracks to claim on KarmaGAP:** "AI Powered Apps & Agents" (primary) + "DeFi & Stablecoin Payments" (use cUSD as the reserve currency for one feature so the Mento angle is honest).

**Why this scores well on POS:**
- Celo-native contracts → transaction count and unique-user metrics actually accumulate
- AI Agents track is the most-funded sponsored prize this season
- Mento stablecoin (cUSD) integration earns the secondary track multiplier
- The agent itself becomes the data source for your KarmaGAP milestones (each signal sale and each yield rebalance is an on-chain event your monthly milestone can point to)
- Open-source, clean architecture, real commit history → the AI quality scorer rewards it

---

## 2. Zero-budget infrastructure choices

| Component | Choice | Cost | Why |
|---|---|---|---|
| LLM for agent decisions | **Google Gemini 2.0 Flash** via free tier (15 req/min, 1M tokens/day) | $0 | Best free quota; rig-core has native Gemini provider |
| Backend hosting | **Fly.io free tier** (3 shared-cpu-1x VMs, 3GB volumes) or **Render free tier** (sleeps after 15 min idle) | $0 | Fly.io supports Rust binaries trivially; volumes for SQLite |
| Database | **SQLite** via `sqlx` SQLite driver, file on Fly volume | $0 | Sufficient for single-instance agent |
| Frontend hosting | **Vercel free tier** (Hobby) | $0 | Native Next.js |
| Celo RPC | **Forno** (`forno.celo.org`) public endpoint, plus Alchemy free tier as backup | $0 | Forno is rate-limited but adequate |
| Celo gas (mainnet) | KarmaGAP sponsors first 5 tx; afterward fund a deployer wallet with ~$2 worth of CELO | <$5 | Celo gas is sub-cent per tx |
| Base/Arbitrum gas (CCTP demo) | Self-funded testnet first; mainnet only for the live demo | <$2 | L2 gas is tiny |
| x402 facilitator | **Coinbase CDP free tier** (1,000 tx/mo) | $0 | Hosted, no infra |
| Signing for verifiable response | **ed25519-dalek** in-process | $0 | Replaces dropped TEE story |
| Indexing/events | Direct Alloy WebSocket subscription, no Goldsky/Subsquid | $0 | One agent, one chain, doesn't need a managed indexer |
| Domain | Vercel free `*.vercel.app` subdomain | $0 | Buy a custom domain only if you place |

**Total cash outlay: under $5** for one-time CELO deployer funding. Everything else uses free tiers.

---

## 3. The Celo-CCTP gap, solved for $0

Since Celo is not a CCTP domain, you have two patterns. Pick **Pattern A** for the hackathon — it stays $0 because the cross-chain leg only fires when there's enough USDC to make it economical. The demo can show a single live execution.

**Pattern A — Earn on Celo, periodically bridge to yield (recommended).**

```
[Buyer agent on any chain]
   │ x402 payment (USDC on Base/Arbitrum/Polygon, OR cUSD on Celo)
   ↓
[BeamRider API on Fly.io]
   │ verifies x402 → returns signed signal → records sale in SQLite
   ↓
[USDC/cUSD accumulates in Celo wallet (and a Base wallet for x402-Base sales)]
   │ scheduler: when balance ≥ threshold AND cross-chain APY delta > gas
   ↓
[YieldStrategy.sol on Celo] — emits StrategyDecided event (transaction-count signal!)
   │
   ├─ If yield winner is on Celo (Aave Celo, Moola, Ubeswap LP):
   │     stay on Celo, deposit directly. One Celo tx per rebalance.
   │
   └─ If yield winner is on Base/Arbitrum:
         Squid/Across moves USDC Celo→Base (one tx, no CCTP for this leg)
         Then CCTP V2 burnWithHook on Base → mints + auto-deposits on dest chain
         (This is your CCTP V2 hooks demo)
```

The CCTP V2 burn-with-hook execution still happens; it just happens *between two non-Celo chains* during the rebalance. You demo it with one live cross-chain rebalance during the pitch video.

**Pattern B — Skip CCTP entirely, keep all yield on Celo.** Use Aave V3 on Celo + Mento + Ubeswap. You lose the CCTP V2 hooks demo but gain simplicity. Reject this for the hackathon: CCTP V2 hooks is the technical novelty that differentiates BeamRider.

---

## 4. Backend architecture (Rust, refined)

### Crate selection (zero-cost, well-maintained)

```toml
[dependencies]
# HTTP server
axum = "0.8"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "limit"] }
tokio = { version = "1", features = ["full"] }

# EVM + CCTP
alloy = { version = "1.0", features = ["full", "ws", "signer-local"] }

# AI agent orchestration — rig-core is the actively-maintained Rust LLM
# framework (0xPlaygrounds), used by Coral Protocol, Nethermind, Dria.
# Has built-in Gemini provider for the free tier and SQLite vector store
# in case you want RAG later.
rig-core = { version = "0.21", features = ["derive"] }

# DB
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "macros", "migrate", "chrono"] }

# x402: parse PAYMENT-REQUIRED / PAYMENT-SIGNATURE headers manually,
# call Coinbase facilitator over HTTP via reqwest. ~80 lines of glue.
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# Crypto for signed responses (replaces TEE)
ed25519-dalek = { version = "2.1", features = ["rand_core"] }

# Boilerplate
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
config = "0.15"
dotenvy = "0.15"
async-trait = "0.1"
```

**Why rig-core over alternatives:** rig is the actively-maintained, Rust-native LLM framework. It exposes a clean `Agent` builder, supports tools, has a Gemini provider (matters for the free tier), supports SQLite as a vector store via the companion `rig-sqlite` crate, and is in production at Coral Protocol, Nethermind, and Dria. The companion `rig-onchain-kit` exists for EVM/Solana interaction; for BeamRider's CCTP flow you'll likely write the calls directly with Alloy because the calldata is small and well-defined, but rig-onchain-kit is worth checking for boilerplate-savings.

### Refined directory layout

The original `auth/` is dropped — for a hackathon, x402 *is* the auth.

```
beamrider-backend/
├── Cargo.toml
├── migrations/                    # sqlx migrations, checked into git
│   ├── 0001_initial.sql
│   ├── 0002_signal_sales.sql
│   └── 0003_rebalance_events.sql
├── src/
│   ├── main.rs                    # tokio runtime + axum bind + worker spawn
│   ├── config.rs                  # AppConfig from env + .env via `config` crate
│   ├── state.rs                   # AppState: db pool, providers, agent handle, signing key
│   ├── error.rs                   # AppError enum + IntoResponse impl (no `errors/` plural)
│   │
│   ├── domain/                    # pure business types — no IO, no async
│   │   ├── mod.rs
│   │   ├── signal.rs              # MarketSignal, SignalKind, Confidence
│   │   ├── payment.rs             # X402Payment, PaymentScheme
│   │   ├── strategy.rs            # YieldStrategy, Venue, ApyQuote
│   │   └── attestation.rs         # SignedResponse (Ed25519)
│   │
│   ├── dto/                       # wire shapes — only Serialize/Deserialize lives here
│   │   ├── mod.rs
│   │   ├── signal_request.rs
│   │   ├── signal_response.rs
│   │   └── x402.rs                # PAYMENT-REQUIRED, PAYMENT-RESPONSE shapes
│   │
│   ├── routes.rs                  # single file: fn router(state) -> Router
│   │
│   ├── handlers/                  # axum handler functions, thin
│   │   ├── mod.rs
│   │   ├── health.rs              # GET /healthz
│   │   ├── signals.rs             # GET /v1/signals/:pair (x402-gated)
│   │   ├── compute.rs             # POST /v1/compute (signed response)
│   │   ├── status.rs              # GET /v1/agent/status (public, for FE)
│   │   └── webhooks.rs            # POST /v1/webhooks/cctp (Iris callback, optional)
│   │
│   ├── middleware/
│   │   ├── mod.rs
│   │   ├── x402.rs                # x402 paywall: returns 402 + verifies via facilitator
│   │   ├── trace.rs               # request-id + tracing layer
│   │   └── ratelimit.rs           # tower-governor; defends free-tier from abuse
│   │
│   ├── services/                  # orchestration: combines repos + chains + agent
│   │   ├── mod.rs
│   │   ├── signal_service.rs      # produces a signal: agent decides → sign → persist
│   │   ├── pricing_service.rs     # fetch current ETH/cUSD/USDC prices (free APIs)
│   │   ├── strategy_service.rs    # picks best yield venue, scores APY net of gas
│   │   └── rebalance_service.rs   # builds the CCTP V2 burnWithHook calldata + submits
│   │
│   ├── repositories/              # SQLite I/O only; trait-based for testability
│   │   ├── mod.rs
│   │   ├── signal_repo.rs         # SignalRepository trait + SqliteSignalRepository
│   │   ├── sale_repo.rs           # x402 payment ledger
│   │   ├── strategy_repo.rs       # last-known APYs, current position
│   │   └── event_repo.rs          # CCTP burn/mint event log
│   │
│   ├── agent/                     # rig-core agent + tools
│   │   ├── mod.rs
│   │   ├── orchestrator.rs        # SignalAgent: prompt + tools, wraps rig::Agent
│   │   ├── tools/
│   │   │   ├── mod.rs
│   │   │   ├── price_tool.rs      # tool: fetch current price (CoinGecko free)
│   │   │   ├── apy_tool.rs        # tool: read APYs from DefiLlama
│   │   │   └── history_tool.rs    # tool: read recent signal history from sqlite
│   │   └── prompts.rs             # system prompts as &'static str, version-tagged
│   │
│   ├── chains/                    # chain-specific code; one module per chain
│   │   ├── mod.rs                 # re-exports + ChainId enum
│   │   ├── celo.rs                # Forno provider, cUSD + USDC ERC20 bindings
│   │   ├── base.rs                # Base provider + USDC + CCTP TokenMessengerV2
│   │   ├── arbitrum.rs            # Arbitrum provider + Aave pool
│   │   └── cctp.rs                # CCTP V2 helpers: addressToBytes32, hookData encoding
│   │
│   ├── workers/                   # long-running tokio tasks; spawned from main
│   │   ├── mod.rs
│   │   ├── earnings_watcher.rs    # WS-subscribe to USDC Transfer to agent wallet
│   │   ├── rebalance_scheduler.rs # tick: check threshold, call rebalance_service
│   │   └── attestation_poller.rs  # poll Iris /v2/messages until attested
│   │
│   ├── crypto/
│   │   ├── mod.rs
│   │   └── signer.rs              # Ed25519 keypair load + sign + verify helpers
│   │
│   └── db.rs                      # SqlitePool builder + migration runner
│
└── tests/
    ├── integration_signal.rs
    ├── integration_x402.rs
    └── integration_cctp_hook.rs   # forks Base via Anvil for hook receiver tests
```

**Two changes from your original layout worth flagging:**

1. **`errors/` → `error.rs`.** A single file holds the error enum + IntoResponse impl. Splitting one error type across a directory adds ceremony without value. Plural-named modules with one type are a Java-ism.

2. **`auth/` removed; `agent/`, `chains/`, `workers/`, `crypto/` added.** x402 is the auth layer (it lives as middleware), so a separate auth/ directory is empty. The new directories reflect what BeamRider actually does: orchestrate an LLM agent, talk to multiple chains, run background workers, sign responses.

**Service / repository boundary, made concrete with one example:**

```rust
// repositories/signal_repo.rs
#[async_trait]
pub trait SignalRepository: Send + Sync {
    async fn insert(&self, s: &domain::Signal) -> Result<i64, AppError>;
    async fn last_n_for_pair(&self, pair: &str, n: i64) -> Result<Vec<domain::Signal>, AppError>;
}

pub struct SqliteSignalRepository { pool: SqlitePool }

#[async_trait]
impl SignalRepository for SqliteSignalRepository {
    async fn insert(&self, s: &domain::Signal) -> Result<i64, AppError> {
        let id = sqlx::query_scalar!(
            "INSERT INTO signals (pair, kind, value_bps, confidence, created_at, signature)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
            s.pair, s.kind as i64, s.value_bps, s.confidence, s.created_at, s.signature
        ).fetch_one(&self.pool).await?;
        Ok(id)
    }
    // ...
}
```

```rust
// services/signal_service.rs
pub struct SignalService<R: SignalRepository, A: agent::Orchestrator> {
    repo: R,
    agent: A,
    signer: crypto::Signer,
}

impl<R: SignalRepository, A: agent::Orchestrator> SignalService<R, A> {
    pub async fn produce(&self, pair: &str) -> Result<domain::SignedResponse, AppError> {
        let history = self.repo.last_n_for_pair(pair, 20).await?;
        let signal = self.agent.decide_signal(pair, &history).await?;  // calls Gemini
        let sig = self.signer.sign(&signal.canonical_bytes());
        let signed = domain::SignedResponse::new(signal.clone(), sig);
        self.repo.insert(&signal).await?;
        Ok(signed)
    }
}
```

Handlers stay thin: parse DTO → call service → map domain → return DTO. Repositories never call other repositories. Services never touch SQL directly. The agent module is just a tool the service uses, like a third-party API.

---

## 5. Smart contracts on Celo

Three contracts. Keep them small — every line is something the AI scorer reads. All deploy to Celo mainnet at the end of week 1.

### `BeamRiderRegistry.sol` (Celo)
- Stores agent metadata: owner address, Ed25519 public key (for response verification), service URL, agent name.
- `registerAgent(...)`, `updateMetadata(...)`, view functions.
- Every registration is a Celo transaction → POS counts it.
- Lightweight ERC-721-style: each agent is a tokenId. Don't claim ERC-8004 compliance unless you implement the full standard; just ship a clean registry.

### `SignalLedger.sol` (Celo)
- Records signal sales as on-chain events (not full storage, to keep gas low).
- `recordSale(bytes32 saleId, address buyer, address agent, bytes32 signalHash, uint256 amount, address token)`.
- Anyone can call but there's a small fee (1 cUSD or 0.1 USDC) that goes to the agent's owner — gives buyers an incentive to record sales (proof of receipt) and accumulates Celo transaction count.
- Optional: include in your demo a "claim discount" path so buyers actually want to record.

### `YieldStrategy.sol` (Celo)
- BeamRider's on-chain treasury and decision log on Celo.
- Holds cUSD/USDC reserve.
- `proposeStrategy(uint8 venue, uint32 destChain, uint256 amount, bytes32 commitHash)` — agent's off-chain decision is committed on-chain before execution. Proposal events are POS-countable.
- `executeStrategyOnCelo(...)` — for staying on Celo (Aave, Ubeswap LP).
- `executeStrategyCrossChain(...)` — for the Squid/Across→CCTP route. Emits `BridgeInitiated`. The actual CCTP burn happens on Base after the Squid hop, so this contract just custody-releases USDC to the bridge router.

### `HookReceiver.sol` (deploy to Base + Arbitrum)
- Implements `IMessageHandlerV2`.
- Receives CCTP-minted USDC + decodes hookData.
- Two actions: `DEPOSIT_AAVE` and `RETURN_HOME` (Squid-back-to-Celo if rebalance fails).
- Uses bytes-extraction helpers from `circlefin/evm-cctp-contracts` for parsing hookData out of the BurnMessage body.
- Critical: enforce `msg.sender == MessageTransmitterV2`, `sourceDomain == EXPECTED_SRC`, `sender == EXPECTED_AGENT_BRIDGE_CONTRACT`. Without these, anyone can deposit dust + a malicious hookData and trigger your contract.

**Mainnet CCTP V2 addresses (deterministic, same on Base + Arbitrum + others):**
- TokenMessengerV2: `0x28b5a0e9C621a5BadaA536219b3a228C8168cf5d`
- MessageTransmitterV2: `0x81D40F21F12A8F0E3252Bccb954D722d4c464B64`

These don't exist on Celo, which is why the bridge leg is Celo→Base (Squid/Across, non-CCTP), then Base→Arbitrum (CCTP V2 with hooks).

---

## 6. SQLite schema (minimal)

```sql
-- 0001_initial.sql
CREATE TABLE agents (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    on_chain_id INTEGER NOT NULL UNIQUE,        -- BeamRiderRegistry tokenId
    owner       TEXT NOT NULL,                  -- 0x... lowercase
    pubkey      BLOB NOT NULL,                  -- Ed25519 32 bytes
    name        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 0002_signal_sales.sql
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

-- 0003_rebalance_events.sql
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
```

---

## 7. Frontend (Next.js, free tier)

App Router, three pages, deployed to Vercel:

- `/` — hero, "buy a signal" demo (connects via wagmi+viem, signs x402 EIP-3009, displays signed signal + Ed25519 pubkey verification badge).
- `/agent` — live BeamRider status: current treasury (cUSD + USDC balance), recent signal sales, current yield position, last rebalance.
- `/dev` — embed of the GitHub repo, contract addresses on Celoscan with verified-source links, Karma GAP project link.

Stack: Next.js 15, wagmi v2 + viem, Tailwind, shadcn/ui. Use the Coinbase x402 JS client for the buyer-side flow if available; otherwise hand-roll the 402 response handling (read response header, sign EIP-3009, retry with header).

Connect a Celo network config so the demo "buy" can be paid in cUSD on Celo (cheapest demo path, also satisfies the Mento track).

---

## 8. Build sequence (revised for $0, solo, ~3-4 weeks of part-time)

POS is monthly. You're not racing a 48-hour clock; you're racing a milestone deadline. This means: ship something runnable on Celo mainnet by week 1 to start accumulating transactions, then iterate.

**Week 1 — Get on Celo mainnet, accumulate transactions early.**
- Deploy `BeamRiderRegistry.sol` and `SignalLedger.sol` to Celo mainnet. Verify on Celoscan.
- Bare Rust agent: axum + sqlx + Gemini via rig. One paid endpoint, one signed signal. Ed25519 signing only.
- Hand-roll x402 middleware against Coinbase facilitator (Base Sepolia first, then Base mainnet).
- Submit BeamRider to KarmaGAP, claim AI Agents + DeFi tracks, write first milestone.

**Week 2 — CCTP V2 hooks demo + Celo yield path.**
- Deploy `HookReceiver.sol` to Base + Arbitrum mainnet. Verify on block explorers.
- Write the rebalance service: Celo→Base via Squid (free SDK), Base CCTP V2 burnWithHook → Arbitrum.
- Deploy `YieldStrategy.sol` to Celo mainnet. Each rebalance proposal is a Celo tx.
- Add Aave V3 Celo and Moola integration for the staying-on-Celo path.

**Week 3 — Frontend, demo loop, Farcaster cast, video.**
- Next.js dashboard.
- End-to-end demo: buyer agent (a CLI script) hits the API → x402 payment → signed signal → signal recorded → eventually rebalance fires.
- 4-min demo video, pitch deck, README cleanup, contract address registration on KarmaGAP.

**Week 4 — Polish, second milestone, submit.**
- Add per-pair signals (ETH-USD, BTC-USD, CELO-USD).
- Stress-test with a few hundred test sales (drives transaction count up).
- Final video, milestone submission.

---

## 9. Honest caveats for $0 mode

- **Gemini 2.0 Flash free tier is rate-limited (15 req/min, 1M tokens/day).** If a buyer hits your API in a burst, the LLM call queues. For a demo this is fine. Add a queue with backpressure if you want polish, otherwise document the limit in the README and have signals cached for 60s per pair so repeat queries hit SQLite, not Gemini.
- **Fly.io's free machine sleeps after idle.** First request after sleep is ~3s cold start. For a demo this is fine; for live load it's not. Mitigate with a cron pinger on uptimerobot.com (free) hitting `/healthz` every 5 min.
- **Forno (Celo public RPC) is rate-limited.** Use it for reads; for writes use a private RPC if you can get one (Alchemy gives a free tier on Celo). Cache `eth_chainId`, `eth_getCode`, etc.
- **Squid SDK is free for low volume but commercial for high volume.** Hackathon demo volume is fine. Alternative: Across (which is free and supports Celo).
- **Coinbase x402 facilitator free tier is 1,000 tx/month.** Plenty for a hackathon.
- **No verifiable compute means buyers must trust BeamRider's registered pubkey.** Be upfront about this in the README. Frame it as "v1 ships with attested signatures; TEE attestation is on the roadmap." Honest. Doesn't bullshit reviewers.
- **POS scoring rewards Celo mainnet transactions specifically.** Mainnet deploys cost real CELO. Budget ~$3-5 in CELO upfront. This is the only unavoidable cost.
- **Don't claim ERC-8004 compliance if you don't implement it.** Reviewers can read source. Just call it `BeamRiderRegistry`.

---

## 10. The single architectural risk to monitor

**The CCTP V2 hooks demo physically lives on Base/Arbitrum, not Celo.** A judge skimming might think "this is a Base project with a Celo logo." Defend against this in the pitch:

- Lead the demo with a Celo signal sale paid in cUSD (Mento track + AI Agents track in one stroke).
- Show the BeamRiderRegistry tokenId on Celoscan.
- Show the SignalLedger event log accumulating on Celoscan.
- THEN show the rebalance: "when yield is better off-Celo, BeamRider autonomously routes via CCTP V2 hooks — here's a live execution."
- Land on: BeamRider's brain lives on Celo; CCTP is one of its tools.

If you can't tell that story crisply in 4 minutes, the project reads as off-topic for POS. The architecture supports the story; the pitch has to match.
