# BeamRider Backend

Rust backend for the BeamRider autonomous signal-selling agent. Built with Axum, Alloy, rig-core, and SQLite.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       axum HTTP Server                       │
│  ┌──────────┐  ┌───────────┐  ┌───────────┐  ┌──────────┐  │
│  │ /healthz │  │ /v1/signals│  │ /v1/compute│  │/v1/status│  │
│  └──────────┘  └─────┬─────┘  └─────┬─────┘  └──────────┘  │
│                      │              │                        │
│          ┌───────────┴──────────────┘                        │
│          │  middleware: x402 │ trace │ ratelimit              │
│          ▼                                                   │
│  ┌───────────────────────────────────────┐                   │
│  │              Services                  │                   │
│  │  signal_service  │ pricing_service     │                   │
│  │  strategy_service│ rebalance_service   │                   │
│  └───────┬──────────┴──────────┬─────────┘                   │
│          │                     │                              │
│  ┌───────▼────────┐  ┌────────▼────────┐                     │
│  │  Repositories   │  │   Agent (rig)    │                    │
│  │  signal_repo    │  │  orchestrator    │                    │
│  │  sale_repo      │  │  tools/          │                    │
│  │  strategy_repo  │  │   price_tool     │                    │
│  │  event_repo     │  │   apy_tool       │                    │
│  └───────┬────────┘  │   history_tool   │                    │
│          │            └────────┬────────┘                     │
│  ┌───────▼────────┐           │                               │
│  │   SQLite (sqlx) │  ┌───────▼────────┐                      │
│  └────────────────┘  │  Gemini Flash   │                      │
│                       └────────────────┘                      │
│                                                               │
│  ┌────────────────────────────────────────────────────┐       │
│  │               Workers (tokio tasks)                 │       │
│  │  earnings_watcher │ rebalance_scheduler │ att_poller│       │
│  └────────────────────────────────────────────────────┘       │
│                                                               │
│  ┌────────────────────────────────────────────────────┐       │
│  │                 Chains (Alloy)                       │       │
│  │  celo.rs │ base.rs │ arbitrum.rs │ cctp.rs          │       │
│  └────────────────────────────────────────────────────┘       │
│                                                               │
│  ┌──────────────┐                                             │
│  │ Crypto (Ed25519) │                                         │
│  └──────────────┘                                             │
└─────────────────────────────────────────────────────────────┘
```

## Module Layout

| Module | Purpose |
|---|---|
| `main.rs` | Tokio runtime, axum bind, worker spawn, graceful shutdown |
| `config.rs` | AppConfig from env + `.env` via `config` crate |
| `state.rs` | AppState: DB pool, providers, agent handle, signing key |
| `error.rs` | AppError enum + IntoResponse impl |
| `routes.rs` | `fn router(state) -> Router` |
| `db.rs` | SQLite pool builder + migration runner |
| `domain/` | Pure business types — no IO, no async |
| `dto/` | Wire shapes — only Serialize/Deserialize |
| `handlers/` | Thin axum handler functions |
| `middleware/` | x402 paywall, tracing, rate limiting |
| `services/` | Orchestration: repos + chains + agent |
| `repositories/` | SQLite I/O; trait-based for testability |
| `agent/` | rig-core agent + tools (Gemini) |
| `chains/` | Per-chain Alloy providers + contract bindings |
| `workers/` | Long-running tokio tasks |
| `crypto/` | Ed25519 keypair, sign, verify |

## Data Flow

```
Buyer Agent ──[x402 payment]──▶ /v1/signals/:pair
                                      │
                  ┌───────────────────┘
                  ▼
           x402 middleware verifies payment via Coinbase facilitator
                  │
                  ▼
           SignalService.produce(pair)
                  │
            ┌─────┴─────┐
            ▼            ▼
    Agent.decide()   Repo.last_n()
    (Gemini Flash)   (SQLite)
            │            │
            └─────┬──────┘
                  ▼
           Signer.sign(signal)
                  │
                  ▼
           Repo.insert(signal)
                  │
                  ▼
           Return SignedResponse ──▶ Buyer Agent
```

## Setup

```bash
# Copy environment template
cp .env.example .env

# Run migrations (auto on startup, or manually)
sqlx migrate run --database-url sqlite:./beamrider.db

# Run in development
cargo run

# Run tests
cargo test
```

## Environment Variables

See `.env.example` for all required configuration.
