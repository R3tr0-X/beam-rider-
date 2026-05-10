// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {Test} from "forge-std/Test.sol";
import {HookReceiver} from "../src/HookReceiver.sol";
import {HookDataCodec} from "../src/libraries/HookDataCodec.sol";
import {AddressUtils} from "../src/libraries/AddressUtils.sol";
import {MockERC20} from "./mocks/MockERC20.sol";
import {MockAavePool} from "./mocks/MockAavePool.sol";

contract HookReceiverTest is Test {
    HookReceiver internal recv;
    MockERC20 internal usdc;
    MockAavePool internal pool;

    address internal owner = makeAddr("owner");
    address internal transmitter = makeAddr("messageTransmitterV2");
    address internal vault = makeAddr("aaveVault");
    address internal returnHome = makeAddr("returnHome");

    uint32 internal constant SRC_DOMAIN = 6;     // example: Base
    bytes32 internal constant SRC_SENDER = bytes32(uint256(0xCAFE));

    uint256 internal constant MINTED = 5_000 * 1e6;

    function setUp() public {
        usdc = new MockERC20("USD Coin", "USDC", 6);
        pool = new MockAavePool();
        recv = new HookReceiver(
            transmitter,
            SRC_DOMAIN,
            SRC_SENDER,
            address(usdc),
            address(pool),
            owner
        );
    }

    // -- constructor ----------------------------------------------------------

    function test_constructor_rejects_zero_args() public {
        vm.expectRevert(HookReceiver.InvalidConstructorArg.selector);
        new HookReceiver(address(0), SRC_DOMAIN, SRC_SENDER, address(usdc), address(pool), owner);

        vm.expectRevert(HookReceiver.InvalidConstructorArg.selector);
        new HookReceiver(transmitter, SRC_DOMAIN, bytes32(0), address(usdc), address(pool), owner);

        vm.expectRevert(HookReceiver.InvalidConstructorArg.selector);
        new HookReceiver(transmitter, SRC_DOMAIN, SRC_SENDER, address(0), address(pool), owner);

        vm.expectRevert(HookReceiver.InvalidConstructorArg.selector);
        new HookReceiver(transmitter, SRC_DOMAIN, SRC_SENDER, address(usdc), address(0), owner);
    }

    // -- DEPOSIT_AAVE ---------------------------------------------------------

    function test_handle_deposit_aave_supplies_pool() public {
        usdc.mint(address(recv), MINTED);
        bytes memory hookData = HookDataCodec.encode(
            HookDataCodec.ACTION_DEPOSIT_AAVE,
            vault,
            bytes("aave-base")
        );

        vm.expectEmit(true, true, false, true);
        emit HookReceiver.Deposited(HookDataCodec.ACTION_DEPOSIT_AAVE, vault, MINTED, bytes("aave-base"));
        vm.prank(transmitter);
        bool ok = recv.handleReceiveFinalizedMessage(SRC_DOMAIN, SRC_SENDER, 2000, hookData);

        assertTrue(ok);
        assertEq(usdc.balanceOf(address(pool)), MINTED);
        assertEq(pool.position(vault, address(usdc)), MINTED);
        assertEq(usdc.allowance(address(recv), address(pool)), 0); // approve was consumed
    }

    function test_handle_return_home_transfers_to_vault() public {
        usdc.mint(address(recv), MINTED);
        bytes memory hookData = HookDataCodec.encode(
            HookDataCodec.ACTION_RETURN_HOME,
            returnHome,
            bytes("rebalance-failed")
        );

        vm.expectEmit(true, false, false, true);
        emit HookReceiver.ReturnedHome(returnHome, MINTED, bytes("rebalance-failed"));
        vm.prank(transmitter);
        bool ok = recv.handleReceiveFinalizedMessage(SRC_DOMAIN, SRC_SENDER, 2000, hookData);

        assertTrue(ok);
        assertEq(usdc.balanceOf(returnHome), MINTED);
    }

    function test_handle_unknown_action_reverts() public {
        usdc.mint(address(recv), MINTED);
        bytes memory hookData = abi.encodePacked(
            uint8(99),
            AddressUtils.addressToBytes32(vault),
            uint32(0)
        );
        vm.prank(transmitter);
        vm.expectRevert(HookReceiver.UnsupportedAction.selector);
        recv.handleReceiveFinalizedMessage(SRC_DOMAIN, SRC_SENDER, 2000, hookData);
    }

    // -- authentication -------------------------------------------------------

    function test_handle_rejects_non_transmitter() public {
        usdc.mint(address(recv), MINTED);
        bytes memory hookData = HookDataCodec.encode(
            HookDataCodec.ACTION_DEPOSIT_AAVE, vault, bytes("")
        );
        vm.expectRevert(HookReceiver.UnauthorizedTransmitter.selector);
        recv.handleReceiveFinalizedMessage(SRC_DOMAIN, SRC_SENDER, 2000, hookData);
    }

    function test_handle_rejects_wrong_source_domain() public {
        usdc.mint(address(recv), MINTED);
        bytes memory hookData = HookDataCodec.encode(
            HookDataCodec.ACTION_DEPOSIT_AAVE, vault, bytes("")
        );
        vm.prank(transmitter);
        vm.expectRevert(HookReceiver.UnexpectedSourceDomain.selector);
        recv.handleReceiveFinalizedMessage(SRC_DOMAIN + 1, SRC_SENDER, 2000, hookData);
    }

    function test_handle_rejects_wrong_sender() public {
        usdc.mint(address(recv), MINTED);
        bytes memory hookData = HookDataCodec.encode(
            HookDataCodec.ACTION_DEPOSIT_AAVE, vault, bytes("")
        );
        vm.prank(transmitter);
        vm.expectRevert(HookReceiver.UnexpectedSender.selector);
        recv.handleReceiveFinalizedMessage(SRC_DOMAIN, bytes32(uint256(0xDEAD)), 2000, hookData);
    }

    // -- unfinalized rejection ------------------------------------------------

    function test_unfinalized_reverts() public {
        bytes memory hookData = HookDataCodec.encode(
            HookDataCodec.ACTION_DEPOSIT_AAVE, vault, bytes("")
        );
        vm.expectRevert(HookReceiver.UnfinalizedNotSupported.selector);
        recv.handleReceiveUnfinalizedMessage(SRC_DOMAIN, SRC_SENDER, 1000, hookData);
    }

    // -- decode failure modes -------------------------------------------------

    function test_handle_rejects_short_hook_data() public {
        usdc.mint(address(recv), MINTED);
        bytes memory tooShort = new bytes(36);
        vm.prank(transmitter);
        vm.expectRevert(HookDataCodec.HookDataTooShort.selector);
        recv.handleReceiveFinalizedMessage(SRC_DOMAIN, SRC_SENDER, 2000, tooShort);
    }

    function test_handle_rejects_zero_vault() public {
        usdc.mint(address(recv), MINTED);
        bytes memory hookData = HookDataCodec.encode(
            HookDataCodec.ACTION_DEPOSIT_AAVE, address(0), bytes("")
        );
        vm.prank(transmitter);
        vm.expectRevert(HookReceiver.InvalidVault.selector);
        recv.handleReceiveFinalizedMessage(SRC_DOMAIN, SRC_SENDER, 2000, hookData);
    }

    function test_handle_rejects_empty_balance() public {
        // No mint to recv: balance is zero.
        bytes memory hookData = HookDataCodec.encode(
            HookDataCodec.ACTION_DEPOSIT_AAVE, vault, bytes("")
        );
        vm.prank(transmitter);
        vm.expectRevert(HookReceiver.TransferFailed.selector);
        recv.handleReceiveFinalizedMessage(SRC_DOMAIN, SRC_SENDER, 2000, hookData);
    }

    // -- rescueToken ----------------------------------------------------------

    function test_rescueToken_owner_only() public {
        MockERC20 tok = new MockERC20("X", "X", 18);
        tok.mint(address(recv), 1 ether);

        vm.expectRevert();
        recv.rescueToken(address(tok), owner, 1 ether);

        vm.prank(owner);
        recv.rescueToken(address(tok), owner, 1 ether);
        assertEq(tok.balanceOf(owner), 1 ether);
    }
}
