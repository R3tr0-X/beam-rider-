// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {Test} from "forge-std/Test.sol";
import {HookDataCodec} from "../src/libraries/HookDataCodec.sol";
import {AddressUtils} from "../src/libraries/AddressUtils.sol";

/// @notice Cross-repo wire-format lock-down for the BeamRider hookData codec.
/// @dev Any change here is breaking against `beamrider-backend/src/chains/cctp.rs`.
contract HookDataCodecTest is Test {
    /// @dev Trampoline that surfaces calldata semantics needed by `decode`.
    function _decode(bytes calldata raw)
        external
        pure
        returns (uint8 action, address vault, bytes memory metadata)
    {
        (uint8 a, address v, bytes calldata m) = HookDataCodec.decode(raw);
        return (a, v, bytes(m));
    }

    function test_encode_layout_matches_rust_format() public pure {
        address vault = 0x1111111111111111111111111111111111111111;
        bytes memory metadata = bytes("aave-arbitrum");
        bytes memory enc = HookDataCodec.encode(HookDataCodec.ACTION_DEPOSIT_AAVE, vault, metadata);

        // 1 byte action
        assertEq(uint8(enc[0]), HookDataCodec.ACTION_DEPOSIT_AAVE);

        // 32 bytes vault: 12 zero bytes ‖ 20-byte address
        for (uint256 i = 1; i <= 12; ++i) assertEq(uint8(enc[i]), 0, "padding nonzero");
        // The last 20 bytes of the vault region are the address.
        bytes20 addrBytes;
        assembly {
            // load word starting at offset 1+12 = 13 (relative to data start)
            addrBytes := mload(add(add(enc, 32), 13))
        }
        assertEq(address(addrBytes), vault);

        // 4 bytes BE u32 length, value 13 (= "aave-arbitrum".length)
        assertEq(uint8(enc[33]), 0);
        assertEq(uint8(enc[34]), 0);
        assertEq(uint8(enc[35]), 0);
        assertEq(uint8(enc[36]), uint8(metadata.length));

        // Tail bytes are the metadata, untouched.
        for (uint256 i = 0; i < metadata.length; ++i) {
            assertEq(uint8(enc[37 + i]), uint8(metadata[i]));
        }

        // Total length matches the formula 37 + N.
        assertEq(enc.length, 37 + metadata.length);
    }

    function test_round_trip_with_metadata() public {
        address vault = makeAddr("vault");
        bytes memory meta = bytes("any-opaque-string");
        bytes memory enc = HookDataCodec.encode(HookDataCodec.ACTION_RETURN_HOME, vault, meta);

        (uint8 a, address v, bytes memory m) = this._decode(enc);
        assertEq(a, HookDataCodec.ACTION_RETURN_HOME);
        assertEq(v, vault);
        assertEq(m, meta);
    }

    function test_round_trip_with_empty_metadata() public {
        address vault = makeAddr("vault");
        bytes memory enc = HookDataCodec.encode(HookDataCodec.ACTION_DEPOSIT_AAVE, vault, bytes(""));
        assertEq(enc.length, 37);

        (uint8 a, address v, bytes memory m) = this._decode(enc);
        assertEq(a, HookDataCodec.ACTION_DEPOSIT_AAVE);
        assertEq(v, vault);
        assertEq(m.length, 0);
    }

    function test_decode_rejects_too_short() public {
        bytes memory raw = new bytes(36);
        vm.expectRevert(HookDataCodec.HookDataTooShort.selector);
        this._decode(raw);
    }

    function test_decode_rejects_length_mismatch() public {
        // header claims 5 bytes of metadata but supplies 0
        bytes memory raw = abi.encodePacked(
            uint8(1),
            AddressUtils.addressToBytes32(makeAddr("vault")),
            uint32(5)
        );
        vm.expectRevert(HookDataCodec.HookDataLengthMismatch.selector);
        this._decode(raw);
    }

    function test_decode_rejects_non_canonical_vault() public {
        // Set a high byte in the upper 12 bytes of the vault region.
        bytes memory raw = abi.encodePacked(
            uint8(1),
            bytes32(uint256(1) << 200),  // top byte non-zero
            uint32(0)
        );
        vm.expectRevert(AddressUtils.NonCanonicalAddress.selector);
        this._decode(raw);
    }

    /// @dev Fuzz the round trip across all action codes / vaults / metadata
    /// payloads. Exhaustively pins the property: decode(encode(x)) == x.
    function testFuzz_round_trip(uint8 action, address vault, bytes memory metadata) public view {
        vm.assume(metadata.length <= 1024); // keep gas reasonable
        bytes memory enc = HookDataCodec.encode(action, vault, metadata);
        (uint8 a, address v, bytes memory m) = this._decode(enc);
        assertEq(a, action);
        assertEq(v, vault);
        assertEq(m, metadata);
    }
}
