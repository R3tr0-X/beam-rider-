// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

/// @notice Minimal single-owner access pattern.
/// @dev Inlined here to keep the contract package free of OpenZeppelin's
/// inheritance graph. Two-step transfer is intentional: a single-tx transfer
/// that fat-fingers an unreachable address is unrecoverable.
abstract contract Ownable {
    address public owner;
    address public pendingOwner;

    error OwnableUnauthorized();
    error OwnableInvalidOwner();
    error OwnableNotPendingOwner();

    event OwnershipTransferStarted(address indexed previousOwner, address indexed newOwner);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    constructor(address initialOwner) {
        if (initialOwner == address(0)) revert OwnableInvalidOwner();
        owner = initialOwner;
        emit OwnershipTransferred(address(0), initialOwner);
    }

    modifier onlyOwner() {
        if (msg.sender != owner) revert OwnableUnauthorized();
        _;
    }

    function transferOwnership(address newOwner) external onlyOwner {
        if (newOwner == address(0)) revert OwnableInvalidOwner();
        pendingOwner = newOwner;
        emit OwnershipTransferStarted(owner, newOwner);
    }

    function acceptOwnership() external {
        address candidate = pendingOwner;
        if (msg.sender != candidate) revert OwnableNotPendingOwner();
        emit OwnershipTransferred(owner, candidate);
        owner = candidate;
        delete pendingOwner;
    }

    function renounceOwnership() external onlyOwner {
        emit OwnershipTransferred(owner, address(0));
        owner = address(0);
        delete pendingOwner;
    }
}
