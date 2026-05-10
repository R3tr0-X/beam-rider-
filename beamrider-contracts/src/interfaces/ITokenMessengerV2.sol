// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

/// @notice Source-chain entry point for Circle CCTP V2 burn-with-hook flows.
/// @dev BeamRider does not call this on-chain (the bridging hop from Celo to a
/// CCTP domain is initiated off-chain via Squid/Across). The interface is
/// retained for ABI documentation and as a typed reference for the off-chain
/// rebalance service.
interface ITokenMessengerV2 {
    function depositForBurnWithHook(
        uint256 amount,
        uint32 destinationDomain,
        bytes32 mintRecipient,
        address burnToken,
        bytes32 destinationCaller,
        uint256 maxFee,
        uint32 minFinalityThreshold,
        bytes calldata hookData
    ) external returns (uint64 nonce);

    function depositForBurn(
        uint256 amount,
        uint32 destinationDomain,
        bytes32 mintRecipient,
        address burnToken,
        bytes32 destinationCaller,
        uint256 maxFee,
        uint32 minFinalityThreshold
    ) external returns (uint64 nonce);
}
