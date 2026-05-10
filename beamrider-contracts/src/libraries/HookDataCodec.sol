// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {AddressUtils} from "./AddressUtils.sol";

/// @notice BeamRider hookData codec.
/// @dev Wire format (must stay byte-identical with the Rust encoder in
/// `beamrider-backend/src/chains/cctp.rs::encode_hook_data`):
///
/// ```
/// [ 1 byte  | action ∈ {1=DEPOSIT_AAVE, 2=RETURN_HOME} ]
/// [ 32 bytes| vault address as bytes32 (12 zero bytes ‖ 20-byte address) ]
/// [ 4 bytes | metadata length, big-endian uint32 ]
/// [ N bytes | metadata, opaque ]
/// ```
library HookDataCodec {
    uint8 internal constant ACTION_DEPOSIT_AAVE = 1;
    uint8 internal constant ACTION_RETURN_HOME = 2;

    uint256 internal constant HEADER_BYTES = 37; // 1 + 32 + 4

    error HookDataTooShort();
    error HookDataLengthMismatch();
    error MetadataTooLarge();

    /// @notice Encode a hook payload. The output is `abi.encodePacked` and
    /// therefore matches the Rust wire format byte-for-byte.
    function encode(
        uint8 action,
        address vault,
        bytes memory metadata
    ) internal pure returns (bytes memory) {
        if (metadata.length > type(uint32).max) revert MetadataTooLarge();
        return abi.encodePacked(
            action,
            AddressUtils.addressToBytes32(vault),
            uint32(metadata.length),
            metadata
        );
    }

    /// @notice Decode a hook payload.
    /// @dev Returns a calldata sub-slice for `metadata` to avoid a memory copy
    /// on the receiver hot path. Reverts on any malformed input.
    function decode(bytes calldata raw)
        internal
        pure
        returns (uint8 action, address vault, bytes calldata metadata)
    {
        if (raw.length < HEADER_BYTES) revert HookDataTooShort();

        action = uint8(raw[0]);
        vault = AddressUtils.bytes32ToAddress(bytes32(raw[1:33]));

        uint32 mlen = uint32(bytes4(raw[33:37]));
        if (raw.length != HEADER_BYTES + uint256(mlen)) revert HookDataLengthMismatch();

        metadata = raw[HEADER_BYTES:];
    }
}
