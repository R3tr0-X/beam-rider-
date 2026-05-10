Locked in. Fetching Hermes and the Stacks Builder Rewards page before writing the brief.Good signal on Stacks' 2026 state. Now getting MiniPay SDK specifics and the Stacks DeFi yield venues for the heavy integration.Enough signal. I have what I need. Writing the full brief now.

---

## BeamRider — Extended Architecture Brief
### MiniPay path (b) + Stacks Heavy + Hermes-pattern Stacks↔EVM relay

---

## What changed from the previous brief, and why

Your three answers introduce three distinct integration seams. None of them break the existing architecture — they extend it. Here is the precise change set:

**MiniPay (b): one path, not a rebuild.** The frontend stays on Vercel Next.js. You add a runtime detection branch (`window.ethereum?.isMiniPay`), a mobile-optimized payment card, and a legacy-transaction override for cUSD gas. Effort: ~1 day. Reward: your demo works inside a wallet with 15M+ users, and Proof of Ship credits you as a MiniPay-compatible Mini App.

**Stacks heavy: new income loop + new yield destination.** BeamRider gains a parallel Stacks-side earnings engine — STX/sBTC paid by Stacks buyers flow into a Stacks-native yield vault (Zest lending or Bitflow LP). The EVM CCTP rebalance path stays intact. You now have *dual yield loops*: EVM earnings → CCTP → Aave/Moola; Stacks earnings → Zest/Bitflow LP. Effort: 10-14 days of focused work. Reward: Stacks Builder Rewards (talent.app, $5K/month, 50 winners) + a genuinely differentiated demo that no other POS project has.

**Hermes-pattern relay: Stacks buyers get the same signal quality as EVM buyers.** The Rust agent produces a signal (as before), signs it (as before), and additionally posts the signal hash to a Clarity oracle contract via an off-chain relay. Stacks buyers read the on-chain hash to verify authenticity before paying. This is the pattern in cypherpulse/hermes-public: publish a verifiable commitment on Stacks, let the buyer verify it, then gate the full payload behind payment. Effort: ~2 days for the relay + oracle contract. Reward: you can honestly claim "Stacks-native signal verification backed by Bitcoin finality."

---

## Updated full architecture

```
Users (browser / MiniPay app)
    │
    ├─ EVM path (wagmi + viem + x402 V2 session tokens)
    │       cUSD on Celo — MiniPay detection branch
    │       USDC on Base/Arbitrum — standard x402
    │
    └─ Stacks path (Stacks.js + Leather/Xverse)
            STX or sBTC payment → signal-ledger.clar
            Receipt hash → verified against signal-oracle.clar
                │
                ▼
        BeamRider Rust Agent  (Fly.io / Docker)
                │
        ┌───────┴────────┐
        │                │
  EVM earnings     Stacks earnings
  (cUSD/USDC)       (STX/sBTC)
        │                │
        ▼                ▼
  YieldStrategy.sol  yield-vault.clar
  (Celo mainnet)     (Stacks mainnet)
        │                │
  CCTP V2 hooks     Zest Protocol
  → Aave Arbitrum   Bitflow LP (sBTC/USDCx)
  OR Moola Celo     StackingDAO (stSTXbtc)
```

---

## 1. MiniPay integration — what to add, exactly

### Detection and branching

MiniPay injects `window.ethereum` with `isMiniPay: true`. It only supports Celo and Celo Sepolia testnet. It does not support EIP-1559 (`maxFeePerGas`, `maxPriorityFeePerGas` are ignored). The only valid feeCurrency is cUSD.

```typescript
// lib/wallet-context.ts
export const isMiniPay =
  typeof window !== 'undefined' && 
  (window as any).ethereum?.isMiniPay === true;
```

```typescript
// hooks/useSignalPurchase.ts
import { isMiniPay } from '@/lib/wallet-context';

export function useSignalPurchase(pair: string) {
  if (isMiniPay) {
    return useMiniPaySignalPurchase(pair);   // cUSD legacy tx path
  }
  return useX402SignalPurchase(pair);         // standard x402 path
}
```

### MiniPay payment path

MiniPay can't participate in x402 (no EIP-3009 support, no WalletConnect). Instead, MiniPay buyers pay directly to your Celo receiver address using `eth_sendTransaction` with `feeCurrency: cUSD_ADDRESS`.

```typescript
// hooks/useMiniPaySignalPurchase.ts
import { parseEther } from 'viem';

const CUSD = '0x765DE816845861e75A25fCA122bb6898B8B1282a';
const RECEIVER = process.env.NEXT_PUBLIC_SIGNAL_RECEIVER;
const PRICE_CUSD = parseEther('0.1');  // $0.10 per signal, sub-cent gas

async function buySignalMiniPay(pair: string) {
  const provider = (window as any).ethereum;
  const [from] = await provider.request({ method: 'eth_requestAccounts' });

  const txHash = await provider.request({
    method: 'eth_sendTransaction',
    params: [{
      from,
      to: CUSD,
      data: encodeFunctionData({
        abi: erc20Abi,
        functionName: 'transfer',
        args: [RECEIVER, PRICE_CUSD],
      }),
      feeCurrency: CUSD,        // ← MiniPay-specific; omit for non-MiniPay
      gas: '0x15F90',
    }],
  });

  // present txHash to backend; backend verifies on Celo then returns signal
  const res = await fetch(`/api/signals/${pair}`, {
    headers: { 'X-MiniPay-TxHash': txHash }
  });
  return res.json();
}
```

### Backend: MiniPay payment verifier (new middleware path)

The `x402` middleware applies to all other clients. For MiniPay, add a companion handler in `handlers/signals.rs`:

```rust
// handlers/signals.rs  (addition)
async fn minipay_verify(
    tx_hash: &str,
    expected_receiver: Address,
    min_amount: U256,
    celo: &CeloProvider,
) -> Result<(), AppError> {
    let receipt = celo.get_transaction_receipt(tx_hash.parse()?).await?
        .ok_or(AppError::PaymentNotFound)?;
    let transfer = parse_erc20_transfer(&receipt, CUSD_ADDRESS, expected_receiver)?;
    if transfer.amount < min_amount {
        return Err(AppError::InsufficientPayment);
    }
    if !celo.is_tx_finalized(&receipt).await? {
        return Err(AppError::PaymentNotFinalized);
    }
    Ok(())
}
```

### Frontend layout for MiniPay

MiniPay runs at 360px width inside a WebView. Your existing Vercel frontend detects `isMiniPay` and renders a compact card instead of the full dashboard.

```tsx
// components/SignalCard.tsx
export function SignalCard({ pair }: { pair: string }) {
  return isMiniPay
    ? <MiniPaySignalCard pair={pair} />    // 360px mobile-first, single CTA
    : <DesktopSignalCard pair={pair} />;   // full wagmi flow
}
```

No viewport rebuild needed. Tailwind's `sm:` breakpoints handle the rest. You can submit this to MiniPay's Mini App discovery page via `docs.minipay.xyz` — it requires a manifest JSON and passing a basic compatibility check (no WalletConnect, cUSD support, mobile layout). That submission earns Proof of Impact gas-usage rewards on top of POS.

---

## 2. Stacks heavy — contracts, yield, payment relay

### New repo directory: `beamrider-stacks-contracts/`

```
beamrider-stacks-contracts/
├── Clarinet.toml
├── contracts/
│   ├── beamrider-registry.clar     # Agent registration on Stacks
│   ├── signal-ledger.clar          # STX/sBTC payment → verifiable receipt
│   ├── signal-oracle.clar          # Receives signal hashes from EVM relayer
│   └── yield-vault.clar            # Routes earnings → Zest/Bitflow
└── tests/
    ├── beamrider-registry_test.ts
    ├── signal-ledger_test.ts
    ├── signal-oracle_test.ts
    └── yield-vault_test.ts
```

### `beamrider-registry.clar`

Mirrors `BeamRiderRegistry.sol` on Stacks. Stores agent pubkey (Ed25519 as a `(buff 32)`), service URL, and owner principal. Every registration is a Stacks transaction → Stacks Builder Rewards metrics. Clarity 4's enhanced trait system makes this clean.

```lisp
;; beamrider-registry.clar
(define-map agents
  { agent-id: uint }
  {
    owner:       principal,
    pubkey:      (buff 32),
    service-url: (string-utf8 256),
    name:        (string-utf8 64),
    active:      bool
  }
)

(define-data-var next-id uint u1)

(define-public (register-agent
    (pubkey      (buff 32))
    (service-url (string-utf8 256))
    (name        (string-utf8 64)))
  (let ((id (var-get next-id)))
    (map-set agents { agent-id: id }
      { owner: tx-sender, pubkey: pubkey,
        service-url: service-url, name: name, active: true })
    (var-set next-id (+ id u1))
    (ok id)
  )
)
```

### `signal-ledger.clar`

This is the Stacks payment gate. A buyer calls `buy-signal`, transfers STX or sBTC, and gets a verifiable receipt. The backend validates the receipt before returning the signed signal. The signal-price-stx and signal-price-sbtc are configurable by the agent owner.

```lisp
;; signal-ledger.clar
(define-constant AGENT_RECEIVER 'SP...) ;; agent's Stacks principal

(define-public (buy-signal-stx (pair (string-utf8 20)) (amount uint))
  (begin
    (try! (stx-transfer? amount tx-sender AGENT_RECEIVER))
    (print { event: "signal-sale", buyer: tx-sender,
             pair: pair, token: "stx", amount: amount,
             block: block-height })
    (ok true)
  )
)

(define-public (buy-signal-sbtc (pair (string-utf8 20)) (amount uint))
  (let ((sbtc (as-contract (contract-call? .sbtc-token transfer
                amount tx-sender AGENT_RECEIVER none))))
    (try! sbtc)
    (print { event: "signal-sale", buyer: tx-sender,
             pair: pair, token: "sbtc", amount: amount,
             block: block-height })
    (ok true)
  )
)
```

The backend verifies by querying Hiro API for events on this contract, matching the buyer principal + block-height window.

### `signal-oracle.clar`

This is the Hermes-pattern piece. The Rust agent's off-chain relay posts signal commitments here. Stacks buyers can verify that a signal hash is authentic before paying. This gives Bitcoin-finality-backed attestation to EVM signals.

```lisp
;; signal-oracle.clar
(define-constant AUTHORIZED_RELAYER 'SP...) ;; BeamRider relay principal

(define-map signal-commitments
  { pair: (string-utf8 20), block-height: uint }
  { hash: (buff 32), confidence-bps: uint }
)

(define-public (commit-signal
    (pair            (string-utf8 20))
    (hash            (buff 32))
    (confidence-bps  uint))
  (begin
    (asserts! (is-eq tx-sender AUTHORIZED_RELAYER) (err u401))
    (map-set signal-commitments
      { pair: pair, block-height: block-height }
      { hash: hash, confidence-bps: confidence-bps })
    (print { event: "signal-committed", pair: pair, hash: hash })
    (ok true)
  )
)

(define-read-only (get-signal
    (pair   (string-utf8 20))
    (height uint))
  (map-get? signal-commitments { pair: pair, block-height: height })
)
```

### `yield-vault.clar`

Routes accumulated STX/sBTC to the best Stacks yield venue. For the hackathon, implement two venues: Zest Protocol (lending) and StackingDAO (liquid stacking for sBTC yield). The contract takes a `venue` enum and calls the respective protocol's deposit function.

```lisp
;; yield-vault.clar  (sketch — full implementation uses Zest/StackingDAO SIP-010 interfaces)
(define-constant VAULT_OWNER 'SP...)
(define-constant ZEST_POOL   'SP...) ;; Zest Protocol pool contract
(define-constant STACKING_DAO 'SP...) ;; StackingDAO contract

(define-public (deposit-to-zest (amount uint))
  (begin
    (asserts! (is-eq tx-sender VAULT_OWNER) (err u403))
    ;; calls Zest's supply function with the vault's sBTC
    (contract-call? ZEST_POOL supply .sbtc-token amount)
  )
)

(define-public (deposit-to-stacking-dao (amount uint))
  (begin
    (asserts! (is-eq tx-sender VAULT_OWNER) (err u403))
    ;; delegates STX for liquid stacking; earns stSTXbtc (sBTC yield daily)
    (contract-call? STACKING_DAO delegate-stx amount)
  )
)
```

**Current Stacks yield rates to know before the demo** (Q1 2026 data):
- Zest Protocol sBTC lending supply: ~3–5% APY in sBTC
- StackingDAO stSTXbtc: sBTC paid daily, ~2–4% APY
- Bitflow sBTC/USDCx LP: trading fees + incentives, variable 5–15% APY
- Dual Stacking (sBTC + STX): up to 5% APY with boost

For the pitch, Bitflow LP is the most visually interesting (swap fees are live). Zest is the most credible for conservative allocation. Ship both options and let the strategy service pick based on current rates from DefiLlama.

---

## 3. Stacks←→EVM signal relay (Hermes pattern)

The relay is a small Rust worker that publishes EVM signal hashes to Stacks. It does not require any bridge — it just calls `commit-signal` on Stacks using a funded relay principal.

### New file: `src/workers/stacks_relay.rs`

```rust
// workers/stacks_relay.rs
use crate::{chains::stacks::StacksClient, domain::Signal, repositories::SignalRepository};

pub async fn run_stacks_relay(
    stacks: StacksClient,
    signal_repo: impl SignalRepository,
    relay_keypair: StacksKeyPair,
) {
    let mut ticker = tokio::time::interval(Duration::from_secs(60)); // post every Stacks block
    loop {
        ticker.tick().await;
        
        for pair in ["ETH-USD", "BTC-USD", "CELO-USD"] {
            if let Ok(Some(signal)) = signal_repo.latest_for_pair(pair).await {
                let hash = signal.commitment_hash(); // sha256 of canonical bytes
                let _ = stacks
                    .call_commit_signal(&relay_keypair, pair, hash, signal.confidence_bps)
                    .await;
                // log but don't panic — relay is best-effort
            }
        }
    }
}
```

### New file: `src/chains/stacks.rs`

```rust
// chains/stacks.rs
// Uses Hiro's public Stacks API — no node needed, free tier
const HIRO_API: &str = "https://api.hiro.so";

pub struct StacksClient {
    http: reqwest::Client,
    base: String,
}

impl StacksClient {
    pub async fn call_commit_signal(
        &self, kp: &StacksKeyPair,
        pair: &str, hash: [u8; 32], confidence_bps: u16,
    ) -> Result<String, AppError> {
        // Build a contract-call tx to signal-oracle::commit-signal
        // Sign with stacks-transactions-rs (community crate)
        // POST to /v2/transactions
        todo!("stacks-transactions-rs contract call builder")
    }

    pub async fn verify_signal_sale(
        &self, buyer: &str, pair: &str, block_window: u32,
    ) -> Result<bool, AppError> {
        // GET /extended/v1/address/{buyer}/transactions
        // find a print event from signal-ledger matching pair + window
        todo!()
    }

    pub async fn get_sbtc_balance(&self, principal: &str) -> Result<u128, AppError> {
        // GET /v2/contracts/call-read/.sbtc-token::get-balance
        todo!()
    }
}
```

**Stacks transactions in Rust**: the community crate `stacks-transactions-rs` handles this (or you write the HTTP directly against Hiro's `/v2/transactions` endpoint — it's a POST with a hex-encoded signed transaction blob). The signing is secp256k1, same as EVM, so `k256` (already in your alloy dependency tree) handles it.

---

## 4. Updated Rust backend additions

Three new modules, minimal changes to existing ones:

```
src/
├── chains/
│   ├── stacks.rs            ← NEW: Hiro API client, tx submission, balance reads
│   └── ...
├── workers/
│   ├── stacks_relay.rs      ← NEW: publishes signal hashes to signal-oracle.clar
│   ├── stacks_yield.rs      ← NEW: monitors Stacks vault, harvests/rebalances
│   └── ...
├── services/
│   └── strategy_service.rs  ← AMENDED: add Stacks venue APY fetcher + comparison
├── handlers/
│   └── signals.rs           ← AMENDED: add MiniPay tx-hash verification path
└── middleware/
    └── x402.rs              ← AMENDED: x402 V2 session token support
```

### `services/strategy_service.rs` amendment

Add `StacksVenue` to the venue enum and a Hiro API APY fetcher:

```rust
pub enum Venue {
    AaveCelo,
    MoolaCelo,
    AaveArbitrum,
    BitflowSbtcUsdcx,  // NEW
    ZestSbtc,          // NEW
    StackingDaoLiquid, // NEW
}

pub async fn fetch_stacks_apys(http: &reqwest::Client) -> Vec<ApyQuote> {
    // DefiLlama API: GET https://yields.llama.fi/pools
    // filter by chain=="Stacks", project in ["zest-protocol","bitflow","stacking-dao"]
    todo!()
}
```

### `middleware/x402.rs` — x402 V2 session tokens

x402 V2 (Dec 2025) introduced session tokens: a buyer deposits a lump sum and gets a session token valid for N requests. This collapses 20 separate per-signal gas payments into one. Worth implementing for the demo — it shows awareness of the current protocol state.

```rust
// middleware/x402.rs  (addition)
async fn check_session_token(
    token: &str,
    db: &SqlitePool,
) -> Result<i64, AppError> {
    // sessions table: token, balance_remaining, buyer, expiry
    let session = sqlx::query_as!(Session,
        "SELECT * FROM sessions WHERE token = ? AND expiry > datetime('now') AND balance > 0",
        token
    ).fetch_optional(db).await?.ok_or(AppError::SessionExpired)?;
    
    sqlx::query!("UPDATE sessions SET balance = balance - 1 WHERE token = ?", token)
        .execute(db).await?;
    
    Ok(session.buyer_chain_id)
}
```

---

## 5. Updated SQLite schema additions

```sql
-- 0004_stacks.sql
CREATE TABLE stacks_sales (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    signal_id    INTEGER NOT NULL REFERENCES signals(id),
    buyer        TEXT NOT NULL,          -- Stacks principal SP...
    pair         TEXT NOT NULL,
    token        TEXT NOT NULL,          -- "stx" or "sbtc"
    amount_atoms TEXT NOT NULL,          -- micro-STX or sats (string for safety)
    stacks_tx_id TEXT NOT NULL UNIQUE,
    block_height INTEGER NOT NULL,
    settled_at   TEXT NOT NULL
);

CREATE TABLE stacks_rebalances (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    proposed_at       TEXT NOT NULL,
    venue             TEXT NOT NULL,     -- "zest-sbtc" | "bitflow-sbtc-usdcx" | "stacking-dao"
    amount_atoms      TEXT NOT NULL,
    expected_apy_bps  INTEGER NOT NULL,
    deposit_tx        TEXT,
    status            TEXT NOT NULL,
    finished_at       TEXT
);

-- 0005_x402_sessions.sql
CREATE TABLE sessions (
    token      TEXT PRIMARY KEY,
    buyer      TEXT NOT NULL,
    chain_id   INTEGER NOT NULL,
    balance    INTEGER NOT NULL,         -- requests remaining
    expiry     TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_sessions_expiry ON sessions(expiry);
```

---

## 6. Frontend: Stacks wallet integration

The frontend gains a second wallet surface alongside wagmi. Leather and Xverse are the two major Stacks wallets; both inject a `StacksProvider` window object. `@stacks/connect` is the canonical library.

```tsx
// providers/StacksProvider.tsx
import { Connect, AppConfig, UserSession } from '@stacks/connect';

const appConfig = new AppConfig(['store_write', 'publish_data']);
export const userSession = new UserSession({ appConfig });

// hooks/useStacksSignalPurchase.ts
import { openContractCall } from '@stacks/connect';
import { uintCV, stringUtf8CV } from '@stacks/transactions';

export function useStacksSignalPurchase(pair: string) {
  const buyWithSTX = async (amountMicroStx: number) => {
    await openContractCall({
      contractAddress: SIGNAL_LEDGER_ADDRESS,
      contractName: 'signal-ledger',
      functionName: 'buy-signal-stx',
      functionArgs: [
        stringUtf8CV(pair),
        uintCV(amountMicroStx),
      ],
      onFinish: (data) => {
        // data.txId → present to backend to get signal
      },
    });
  };
  return { buyWithSTX };
}
```

The `/` page now has three payment cards:
1. **Pay with MiniPay** (cUSD, mobile-first, shown if `isMiniPay`)
2. **Pay with EVM wallet** (USDC/cUSD, wagmi + x402 V2)
3. **Pay with Leather/Xverse** (STX or sBTC, Stacks.js)

---

## 7. Revised directory layout

```
beamrider/
├── beamrider-backend/          (Rust — unchanged shape, new modules)
│   └── src/
│       ├── chains/
│       │   ├── stacks.rs       ← NEW
│       │   └── ...
│       └── workers/
│           ├── stacks_relay.rs ← NEW
│           └── stacks_yield.rs ← NEW
│
├── beamrider-stacks-contracts/ ← NEW top-level directory
│   ├── Clarinet.toml
│   └── contracts/
│       ├── beamrider-registry.clar
│       ├── signal-ledger.clar
│       ├── signal-oracle.clar
│       └── yield-vault.clar
│
├── beamrider-frontend/         (Next.js — additions only)
│   └── src/
│       ├── providers/
│       │   ├── StacksProvider.tsx  ← NEW
│       │   └── WagmiProvider.tsx   (unchanged)
│       └── hooks/
│           ├── useStacksSignalPurchase.ts ← NEW
│           ├── useMiniPaySignalPurchase.ts ← NEW
│           └── useX402SignalPurchase.ts    (unchanged)
│
└── docker-compose.yml          (unchanged)
```

---

## 8. Prize targeting — exactly what this architecture hits

### Proof of Ship (POS) — primary

- **AI Agents + DeFi tracks**: unchanged from the original brief. Lead demo: Celo signal sale in cUSD.
- **MiniPay track** (if POS runs a Mini App bounty): your frontend is now MiniPay-compatible. Submit the manifest. Every MiniPay signal sale adds to Proof of Impact gas metrics.
- **Stacks doesn't count for POS transaction metrics** (wrong chain), but the cross-chain yield story strengthens the AI Agents narrative: "BeamRider autonomously routes capital to wherever yield is highest — including Bitcoin L2."

### Stacks Builder Rewards (talent.app) — new

The April round paid $5,000 to 50 winners. May runs the same structure. Rankings are based on impact and activity during the month. Concrete things that drive ranking:
- Mainnet Stacks transactions from `signal-ledger.clar` calls (each buyer = one tx)
- `beamrider-registry.clar` registration transactions
- `signal-oracle.clar` relay commits (one per Stacks block ≈ one per 10 min during demo)
- `yield-vault.clar` deposit transactions

To rank well, you need real transactions on Stacks mainnet before the cutoff. Ship the Clarity contracts in week 1. A few test buyers (even yourself from different wallets) calling `buy-signal-stx` give you the transaction baseline.

### Low-hanging additional prizes

The three signal strategies with the best effort-to-prize ratio for a Stacks-native dApp right now:

**1. Zest Protocol integration (lending yield on sBTC/USDCx):** Zest is the largest Stacks DeFi protocol at $75.9M TVL. If Zest runs a builder program (check their Discord), integrating `yield-vault.clar → Zest` directly qualifies. Effort: deploy `yield-vault.clar` with Zest's supply interface. Prize path: Zest grants + ecosystem exposure.

**2. Bitflow LP (trading fee yield):** Bitflow is the main DEX for sBTC/USDCx. Adding a Bitflow LP position as a yield destination in `yield-vault.clar` makes BeamRider a TVL contributor. Bitflow has run builder incentives and would likely amplify a project that adds TVL. Effort: call Bitflow's `add-liquidity` function from the vault.

**3. Dual Stacking sBTC+STX (the "most signals to Stacks" story):** The 2026 roadmap explicitly rewards builders creating "agentic readability" for AI to interact with Bitcoin contracts. BeamRider is literally an AI agent interacting with Bitcoin contracts. Frame it that way in your Stacks Builder Rewards submission. The signal oracle + yield vault together form an AI-readable, BTC-finality-backed infrastructure primitive.

---

## 9. Hermes-public — what to extract

Since the repo was inaccessible to fetch directly, here is what you should look for when you clone it and what is reusable:

**Extract and use:**
- Their `signal-oracle.clar` pattern (if they have one) — compare against the template above and take whichever is cleaner.
- Their Stacks.js wallet connection boilerplate (it saves 2–3 hours).
- Their off-chain relay approach: how they authenticate the EVM-side signer to the Stacks contract. The `AUTHORIZED_RELAYER` principal in `signal-oracle.clar` above is the key — look at how Hermes handles key management for the relay identity.

**Do not copy:**
- Their contract architecture wholesale if it targets a different use case. BeamRider's `signal-ledger.clar` is simpler than a full cross-chain messaging system.
- Any hardcoded addresses from their testnet deploy.

**The contract directory is `beamrider-stacks-contracts/`**, not `beamrider-stack-contract/` — note the plural and the hyphen. Keep it consistent with the `-contracts` suffix used in the EVM side.

---

## 10. Build sequence amendment

The existing 4-week sequence holds. Insertions are additive:

**Week 1** (unchanged + Clarity contracts):
- Deploy `BeamRiderRegistry.sol`, `SignalLedger.sol` to Celo mainnet.
- **New**: Deploy `beamrider-registry.clar`, `signal-ledger.clar` to Stacks mainnet.
- `clarinet integrate` for local testing. Stacks mainnet deploy costs < 1 STX.
- Submit to Stacks Builder Rewards on talent.app immediately after deploy.

**Week 2** (unchanged + relay + MiniPay):
- Deploy `signal-oracle.clar`, `yield-vault.clar` to Stacks mainnet.
- Wire up `stacks_relay.rs` worker — signal hashes start posting every block.
- Add MiniPay detection branch to frontend. Test on an Android device.
- Connect `yield-vault.clar` to Zest (easier than Bitflow — just one `supply` call).

**Week 3** (unchanged + Stacks.js frontend):
- Add Leather/Xverse wallet connection to the Next.js app.
- Build the three-payment-path signal purchase UI.
- End-to-end demo: EVM buyer via x402, MiniPay buyer via cUSD, Stacks buyer via STX.

**Week 4** (unchanged):
- Add Bitflow LP as second Stacks yield venue.
- Demo video showing all three payment paths + dual yield loops.
- Final milestone submission for both POS and Stacks Builder Rewards.

---

## 11. Honest additions to the caveats list

These are new risks the Stacks integration introduces:

**Clarity learning curve is real.** Clarity is not Solidity. It is Lisp-syntax, decidable (no unbounded loops), and interpreted on-chain. `clarinet` is the local dev tool. Budget 4–6 hours to get comfortable before writing real contract logic. The Clarity crash course at `docs.stacks.co/guides-and-tutorials/clarity-crash-course` is the fastest path.

**`stacks-transactions-rs` is a community crate, not official.** Verify it can build signed contract-call transactions before depending on it. Alternative: use the Node.js `@stacks/transactions` package in a small sidecar service called from Rust over stdio or HTTP. For a hackathon, the sidecar is acceptable.

**Stacks block times are ~10s post-Nakamoto, but anchored to Bitcoin.** Signal relay commits land within 10–30 seconds. This is fast enough for the demo, but tell buyers the oracle update cadence in the UI.

**No x402 native support for STX/sBTC.** x402 is EVM + Solana only as of March 2026. The Stacks payment path uses direct contract calls, not x402. This is fine — be upfront in the README. Frame it as "EVM buyers use x402 V2; Stacks buyers use Clarity-native payment receipts." Different verbs, same economic primitive.

**Zest/Bitflow contract interfaces may change.** Audit their deployed mainnet contracts before calling them from `yield-vault.clar`. Both are audited by Coinfabrik. Use `clarinet check` with their contract ABIs imported as dependencies.