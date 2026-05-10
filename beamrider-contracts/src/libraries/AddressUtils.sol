// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

/// @notice Bidirectional conversion between 20-byte EVM addresses and the
/// 32-byte left-padded form used throughout CCTP V2 message bodies.
library AddressUtils {
    /// @dev Reverts when a `bytes32` whose top 12 bytes are non-zero is decoded.
    /// CCTP V2 only ever produces canonical (zero-padded) values; non-canonical
    /// inputs indicate a corrupted or hostile message.
    error NonCanonicalAddress();

    function addressToBytes32(address account) internal pure returns (bytes32) {
        return bytes32(uint256(uint160(account)));
    }

    function bytes32ToAddress(bytes32 word) internal pure returns (address) {
        if (uint256(word) >> 160 != 0) revert NonCanonicalAddress();
        return address(uint160(uint256(word)));
    }
}
