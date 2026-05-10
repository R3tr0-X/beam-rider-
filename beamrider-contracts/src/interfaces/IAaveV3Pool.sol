// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

/// @notice Minimal Aave V3 pool surface used by BeamRider's HookReceiver.
interface IAaveV3Pool {
    function supply(
        address asset,
        uint256 amount,
        address onBehalfOf,
        uint16 referralCode
    ) external;

    function withdraw(
        address asset,
        uint256 amount,
        address to
    ) external returns (uint256);
}
