// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {IERC20} from "../../src/interfaces/IERC20.sol";

/// @notice Test-only ERC-20. Kept minimal: enough surface for the BeamRider
/// contracts that consume `IERC20`. `mint` is unrestricted on purpose.
contract MockERC20 is IERC20 {
    string public name;
    string public symbol;
    uint8 public immutable override decimals;

    uint256 public override totalSupply;
    mapping(address => uint256) public override balanceOf;
    mapping(address => mapping(address => uint256)) public override allowance;

    /// @notice When `true`, every `transfer` / `transferFrom` returns `false`
    /// without state changes. Used to exercise the `TransferFailed` paths.
    bool public failTransfers;

    constructor(string memory name_, string memory symbol_, uint8 decimals_) {
        name = name_;
        symbol = symbol_;
        decimals = decimals_;
    }

    function mint(address to, uint256 amount) external {
        totalSupply += amount;
        balanceOf[to] += amount;
        emit Transfer(address(0), to, amount);
    }

    function setFailTransfers(bool fail) external {
        failTransfers = fail;
    }

    function approve(address spender, uint256 amount) external override returns (bool) {
        allowance[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function transfer(address to, uint256 amount) external override returns (bool) {
        if (failTransfers) return false;
        _transfer(msg.sender, to, amount);
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external override returns (bool) {
        if (failTransfers) return false;
        uint256 a = allowance[from][msg.sender];
        require(a >= amount, "MockERC20: allowance");
        unchecked { allowance[from][msg.sender] = a - amount; }
        _transfer(from, to, amount);
        return true;
    }

    function _transfer(address from, address to, uint256 amount) private {
        uint256 bal = balanceOf[from];
        require(bal >= amount, "MockERC20: balance");
        unchecked { balanceOf[from] = bal - amount; }
        balanceOf[to] += amount;
        emit Transfer(from, to, amount);
    }
}
