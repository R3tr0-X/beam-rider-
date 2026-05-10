// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

/// @notice Minimal ERC-20 surface used by BeamRider contracts.
/// @dev Targets are USDC and Celo cUSD, both of which return `bool` and adhere
/// to the standard. We deliberately do not pull in OpenZeppelin to keep the
/// dependency tree at zero.
interface IERC20 {
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    function totalSupply() external view returns (uint256);
    function balanceOf(address account) external view returns (uint256);
    function allowance(address owner, address spender) external view returns (uint256);
    function decimals() external view returns (uint8);

    function transfer(address to, uint256 amount) external returns (bool);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
}
