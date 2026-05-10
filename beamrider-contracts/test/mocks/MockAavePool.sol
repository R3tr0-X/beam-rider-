// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {IAaveV3Pool} from "../../src/interfaces/IAaveV3Pool.sol";
import {IERC20} from "../../src/interfaces/IERC20.sol";

/// @notice Test stand-in for Aave V3 Pool. `supply` pulls the asset from the
/// caller using the previously-set allowance and credits a flat 1:1 internal
/// position to `onBehalfOf`. `withdraw` reverses it. No interest, no liquidity
/// index, no aTokens — only enough state for assertions.
contract MockAavePool is IAaveV3Pool {
    mapping(address => mapping(address => uint256)) public position; // user -> asset -> amount

    event SupplyCalled(address indexed asset, address indexed onBehalfOf, uint256 amount, uint16 referralCode);

    function supply(address asset, uint256 amount, address onBehalfOf, uint16 referralCode) external override {
        bool ok = IERC20(asset).transferFrom(msg.sender, address(this), amount);
        require(ok, "MockAavePool: transferFrom");
        position[onBehalfOf][asset] += amount;
        emit SupplyCalled(asset, onBehalfOf, amount, referralCode);
    }

    function withdraw(address asset, uint256 amount, address to) external override returns (uint256) {
        uint256 p = position[msg.sender][asset];
        uint256 send = amount > p ? p : amount;
        position[msg.sender][asset] = p - send;
        bool ok = IERC20(asset).transfer(to, send);
        require(ok, "MockAavePool: transfer");
        return send;
    }
}
