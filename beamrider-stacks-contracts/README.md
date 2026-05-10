# beamrider-stacks-contracts

Clarity contracts for the Stacks-side of BeamRider. Mirrors the four
Solidity contracts in `../beamrider-contracts/` plus adds the
Hermes-pattern signal oracle.

## Contracts

| Contract | Solidity counterpart | Purpose |
|---|---|---|
| `beamrider-registry` | `BeamRiderRegistry.sol` | Agent identity. `agent-id` MUST equal the Celo `tokenId` for the same agent. |
| `signal-ledger` | `SignalLedger.sol` | STX / SIP-010 sale receipts. Replay-protected on `sale-id`. |
| `signal-oracle` | (none) | Hermes-pattern hash commitments. Authorized relayer posts SHA-256 of canonical signal bytes. |
| `yield-vault` | `YieldStrategy.sol` (loosely) | Custody + dispatch to Zest, Bitflow LP, StackingDAO. No generic call surface. |

## Trust model

- **Owner principal** is single-step transferable on each contract. Two-step
  is intentionally omitted on Stacks: post-Nakamoto block times are short and
  the off-chain operator can re-deploy from scratch faster than recovering
  from a fat-fingered owner on Stacks mainnet.
- **Authorized relayer principal** on `signal-oracle.clar` is custodial. The
  relayer key signs Stacks transactions to commit signal hashes. **Lose this
  key and an attacker can post arbitrary commitments**. Mitigation: rotate
  via `set-relayer` and re-deploy the off-chain relay worker.
- **`agent-id` and Celo `tokenId` are operator-asserted to match.** A drift
  between chains is a documentation/indexing issue, not a security one — the
  Ed25519 pubkey itself is the source of authenticity for any signed signal.

## Verification

```bash
clarinet check
clarinet console     # interactive REPL
clarinet integrate   # devnet w/ Bitcoin anchoring
```

Tests live under `tests/`.

## Deploy

Deploy in this order on Stacks mainnet:

1. `beamrider-registry`
2. `signal-ledger`
3. `signal-oracle`
4. `yield-vault`

After step 3, set the relayer principal via `(contract-call? .signal-oracle set-relayer 'SP…)`.
After step 4, set venue targets via `set-venue-target` per venue id (1=Zest, 2=Bitflow LP, 3=StackingDAO).

## Wire-format invariant

The `signal-oracle.clar` `commit-signal` `hash` is **SHA-256 over the
canonical signal bytes** as produced by
`beamrider-backend/src/domain/signal.rs::MarketSignal::canonical_bytes`.
Keep these byte-identical: changing the canonical encoding requires bumping
both sides + a backfill of historical commitments.
