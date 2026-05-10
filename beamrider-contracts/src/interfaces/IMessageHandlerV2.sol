// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

/// @notice Circle CCTP V2 receiver callback.
/// @dev Invoked by the local `MessageTransmitterV2` after USDC is minted to the
/// recipient. `messageBody` is the hookData payload supplied to the source-chain
/// `depositForBurnWithHook` call.
interface IMessageHandlerV2 {
    function handleReceiveFinalizedMessage(
        uint32 sourceDomain,
        bytes32 sender,
        uint256 finalityThresholdExecuted,
        bytes calldata messageBody
    ) external returns (bool);

    function handleReceiveUnfinalizedMessage(
        uint32 sourceDomain,
        bytes32 sender,
        uint256 finalityThresholdExecuted,
        bytes calldata messageBody
    ) external returns (bool);
}
