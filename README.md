# BeamRider

An autonomous agent on Celo that sells price/market signals to other agents over x402, accepts payment in cUSD/USDC, and routes earnings cross-chain via CCTP V2 hooks to wherever yield is best.

## Project Structure

```
beam-rider/
├── beamrider-brief.md          # Project brief and specification
├── beamrider-contracts/        # Solidity smart contracts (Foundry)
│   ├── src/                    # Contract sources
│   ├── test/                   # Foundry tests
│   ├── script/                 # Deploy scripts
│   └── README.md
├── beamrider-backend/          # Rust backend (Axum + Alloy + rig-core)
│   ├── src/                    # Application source
│   ├── migrations/             # SQLite migrations
│   ├── tests/                  # Integration tests
│   └── README.md
└── README.md                   # This file
```

## Tracks

- **Primary:** AI Powered Apps & Agents
- **Secondary:** DeFi & Stablecoin Payments (cUSD / Mento)

## Architecture Overview

BeamRider's brain lives on Celo. CCTP is one of its tools.

1. **Signal Sales** — Buyer agents pay via x402 (USDC on Base/Arbitrum/Polygon, or cUSD on Celo) → receive Ed25519-signed market signals
2. **On-Chain Registry** — Agent identity and metadata stored as lightweight ERC-721-style tokenIds on Celo
3. **Signal Ledger** — Every sale recorded as a Celo event (POS transaction count)
4. **Yield Routing** — Earnings automatically deployed to best-yield venue:
   - On-Celo: Aave V3 Celo, Moola, Ubeswap LP
   - Cross-chain: Celo→Base (Squid/Across) → CCTP V2 burnWithHook → Arbitrum (Aave)

## Quick Start

```bash
# Smart contracts
cd beamrider-contracts && forge build && forge test

# Backend
cd beamrider-backend && cargo run
```

See individual README files in each package for detailed setup instructions.
