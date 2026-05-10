# BeamRider Agent Index

BeamRider is a Celo-first autonomous agent that sells signed market signals over x402 and records enough on-chain/off-chain state to support later yield rebalancing through CCTP V2 hooks.

## Repository Map

- `beamrider-brief.md` is the product and architecture source of truth.
- `README.md` gives the top-level project story and package layout.
- `beamrider-backend/` is the Rust MVP: Axum, sqlx SQLite, Ed25519 signatures, x402, and typed CCTP helpers.
- `beamrider-contracts/` is the Foundry workspace for the Celo registry/ledger/strategy and L2 hook receiver.

## Backend Rules

- x402 is the auth layer. Do not add a parallel auth subsystem for the MVP.
- Handlers stay thin: parse HTTP inputs, call services, return DTOs.
- Domain modules stay pure: no SQL, no HTTP, no async.
- Repositories own SQLite I/O. Services orchestrate repositories, agent logic, signing, and chain helpers.
- Prefer concrete runtime types and static dispatch. Use traits only at external boundaries that need test fakes.
- SQLite is a single-node production database here: use WAL for file databases, foreign keys, busy timeout, bounded pools, and indexed lookups.
- Store token amounts as decimal strings when values may exceed integer-safe JSON/SQLite ranges.
- Keep Celo cUSD distinct from CDP/x402 USDC support. Do not claim CDP facilitator support for Celo cUSD until a real verifier exists.

## Contracts Layout

Foundry workspace at `beamrider-contracts/`. Solidity `0.8.25`, optimizer 200, `via_ir = false`.

### Source files

- `src/BeamRiderRegistry.sol` — agent identity registry. Each agent is a `tokenId` with `(owner, ed25519PubKey, name, serviceUrl)`. Not ERC-721 compliant; deliberately minimal.
- `src/SignalLedger.sol` — sale receipts. `recordSale(saleId, agentTokenId, signalHash, feeToken, paidAmount)` pulls a per-token `minFee` from the buyer to the agent owner. `paidAmount` is the off-chain x402 amount; logged but not transferred.
- `src/YieldStrategy.sol` — Celo treasury + decision log. `proposeStrategy → executeStrategyOnCelo | executeStrategyCrossChain`. Cross-chain path simply transfers ERC-20 custody to the configured bridge router; the actual Squid/CCTP call lives off-chain.
- `src/HookReceiver.sol` — destination-chain `IMessageHandlerV2` implementor. Decodes the BeamRider hookData wire format, then either supplies USDC to Aave V3 or transfers it to a configured return address. Rejects unfinalized messages.

### Libraries and interfaces

- `src/libraries/AddressUtils.sol` — `addressToBytes32` / `bytes32ToAddress` with canonical-address validation.
- `src/libraries/HookDataCodec.sol` — wire-format encode/decode. **The wire format MUST stay byte-identical with `beamrider-backend/src/chains/cctp.rs::encode_hook_data`** (see invariant below).
- `src/interfaces/IMessageHandlerV2.sol` — Circle CCTP V2 receiver callback (finalized + unfinalized variants).
- `src/interfaces/ITokenMessengerV2.sol` — minimal `depositForBurnWithHook` declaration; not consumed on-chain by BeamRider, kept for ABI documentation.
- `src/interfaces/IAaveV3Pool.sol` — `supply` / `withdraw` only.
- `src/interfaces/IERC20.sol` — minimal ERC-20 used by every contract here.
- `src/utils/Ownable.sol` — small inline owner pattern; reused by `SignalLedger`, `YieldStrategy`, `HookReceiver`.

### Tests

`forge test` runs locally with no env vars. Each contract has a focused suite under `test/` (`*.t.sol`). `HookReceiver.t.sol` ships in-file mock USDC and mock Aave pool — no fork.

### Deploy scripts

- `script/DeployCelo.s.sol` deploys `BeamRiderRegistry`, `SignalLedger`, `YieldStrategy` on Celo mainnet.
- `script/DeployHookReceiver.s.sol` deploys `HookReceiver` on Base / Arbitrum.
- Both read `DEPLOYER_PRIVATE_KEY` from env and rely on the `[rpc_endpoints]` section of `foundry.toml`.

### Cross-Repo Wire-Format Invariant

The CCTP hookData wire format is shared across two implementations and changes are breaking on both sides:

```
[ 1 byte  | action ∈ {1=DEPOSIT_AAVE, 2=RETURN_HOME} ]
[ 32 bytes| vault address as bytes32 (12 zero bytes ‖ 20-byte address) ]
[ 4 bytes | metadata length, big-endian uint32 ]
[ N bytes | metadata, opaque ]
```

Reference Rust encoder: `beamrider-backend/src/chains/cctp.rs::encode_hook_data`.
Reference Solidity codec: `beamrider-contracts/src/libraries/HookDataCodec.sol`.

If you change one, change the other and add a round-trip test on both sides.

## Performance And Maintenance

- Avoid global locks on request paths.
- Avoid per-request heavyweight client construction where a cloneable client or config object is enough.
- Avoid speculative abstractions, unused workers, or live chain transaction code before credentials and tests exist.
- Keep route/service behavior deterministic when external credentials are absent.
- In Solidity: use custom `error` types over revert strings; avoid OpenZeppelin or other dependency trees; use `immutable` for constructor-time addresses; follow checks-effects-interactions and skip reentrancy guards when CEI is sufficient.

## Test Commands

Run from `beamrider-backend/`:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Run from `beamrider-contracts/`:

```bash
forge build
forge test
```

The local backend must start with only `DATABASE_URL` configured. Missing Gemini, RPC, or wallet values should disable live external behavior, not break local tests. The Foundry suite must pass with no env vars set.
