# BeamRider Contracts

Solidity smart contracts for the BeamRider autonomous signal-selling agent. Built with [Foundry](https://book.getfoundry.sh/).

## Architecture

### Celo Mainnet Contracts

| Contract | Purpose |
|---|---|
| `BeamRiderRegistry.sol` | On-chain agent identity registry. Stores owner, Ed25519 pubkey, service URL, name. Lightweight ERC-721-style tokenId per agent. |
| `SignalLedger.sol` | Records signal sales as on-chain events. Buyers call `recordSale()` with a small fee (1 cUSD / 0.1 USDC) that routes to the agent owner. |
| `YieldStrategy.sol` | On-chain treasury + decision log. Holds cUSD/USDC, proposes strategies (commit-hash on-chain before execution), executes on-Celo or cross-chain rebalances. |

### L2 Contracts (Base + Arbitrum)

| Contract | Purpose |
|---|---|
| `HookReceiver.sol` | Implements `IMessageHandlerV2` for CCTP V2 hook reception. Decodes `hookData` → `DEPOSIT_AAVE` or `RETURN_HOME` actions. Enforces `msg.sender == MessageTransmitterV2`, `sourceDomain`, `sender` checks. |

### Libraries

| Library | Purpose |
|---|---|
| `HookDataCodec.sol` | Encode/decode CCTP V2 hookData payloads (action type, destination vault, metadata). |
| `AddressUtils.sol` | `addressToBytes32` / `bytes32ToAddress` helpers required by CCTP V2 message format. |

### Interfaces

| Interface | Purpose |
|---|---|
| `IMessageHandlerV2.sol` | CCTP V2 callback interface that `HookReceiver` implements. |
| `ITokenMessengerV2.sol` | CCTP V2 `depositForBurnWithHook` callsite. |
| `IAaveV3Pool.sol` | Aave V3 `supply` / `withdraw` for yield deposits. |
| `IERC20.sol` | Minimal ERC20 for USDC/cUSD interactions. |

## Mainnet CCTP V2 Addresses (Deterministic)

```
TokenMessengerV2:      0x28b5a0e9C621a5BadaA536219b3a228C8168cf5d
MessageTransmitterV2:  0x81D40F21F12A8F0E3252Bccb954D722d4c464B64
```

> These exist on Base + Arbitrum. They do **not** exist on Celo — which is why the bridge leg is Celo→Base (Squid/Across), then Base→Arbitrum (CCTP V2 with hooks).

## Setup

```bash
# Install dependencies
forge install

# Build
forge build

# Test
forge test

# Deploy to Celo mainnet
forge script script/DeployCelo.s.sol --rpc-url celo --broadcast --verify

# Deploy HookReceiver to Base
forge script script/DeployHookReceiver.s.sol --rpc-url base --broadcast --verify

# Deploy HookReceiver to Arbitrum
forge script script/DeployHookReceiver.s.sol --rpc-url arbitrum --broadcast --verify
```

## Environment Variables

Copy `.env.example` → `.env` and fill in:

```bash
cp .env.example .env
```
