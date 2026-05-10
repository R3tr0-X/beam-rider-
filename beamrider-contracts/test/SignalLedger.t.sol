// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {Test} from "forge-std/Test.sol";
import {SignalLedger} from "../src/SignalLedger.sol";
import {BeamRiderRegistry} from "../src/BeamRiderRegistry.sol";
import {MockERC20} from "./mocks/MockERC20.sol";

contract SignalLedgerTest is Test {
    BeamRiderRegistry internal reg;
    SignalLedger internal ledger;
    MockERC20 internal cusd;
    MockERC20 internal usdc;

    address internal admin = makeAddr("admin");
    address internal agentOwner = makeAddr("agentOwner");
    address internal buyer = makeAddr("buyer");
    address internal stranger = makeAddr("stranger");

    uint256 internal constant CUSD_FEE = 1e18;          // 1 cUSD (18 dec)
    uint256 internal constant USDC_FEE = 1e5;           // 0.1 USDC (6 dec)
    uint256 internal agentId;

    bytes32 internal constant SALE_ID  = keccak256("sale-1");
    bytes32 internal constant SIG_HASH = keccak256("signal-1");

    function setUp() public {
        reg = new BeamRiderRegistry();
        ledger = new SignalLedger(address(reg), admin);

        cusd = new MockERC20("Celo Dollar", "cUSD", 18);
        usdc = new MockERC20("USD Coin", "USDC", 6);

        vm.startPrank(admin);
        ledger.setAllowedToken(address(cusd), CUSD_FEE);
        ledger.setAllowedToken(address(usdc), USDC_FEE);
        vm.stopPrank();

        vm.prank(agentOwner);
        agentId = reg.registerAgent(keccak256("pk"), "alpha", "https://a");

        cusd.mint(buyer, 100 ether);
        usdc.mint(buyer, 100 * 1e6);
    }

    // -- constructor ----------------------------------------------------------

    function test_constructor_rejects_zero_registry() public {
        vm.expectRevert(SignalLedger.InvalidRegistry.selector);
        new SignalLedger(address(0), admin);
    }

    // -- setAllowedToken ------------------------------------------------------

    function test_setAllowedToken_only_owner() public {
        vm.expectRevert();
        ledger.setAllowedToken(address(cusd), 2e18);
    }

    function test_setAllowedToken_zero_disallows() public {
        vm.prank(admin);
        ledger.setAllowedToken(address(cusd), 0);
        assertEq(ledger.minFee(address(cusd)), 0);
    }

    // -- recordSale -----------------------------------------------------------

    function test_recordSale_charges_fee_to_agent_owner() public {
        vm.prank(buyer);
        cusd.approve(address(ledger), CUSD_FEE);

        vm.prank(buyer);
        ledger.recordSale(SALE_ID, agentId, SIG_HASH, address(cusd), 5e18);

        assertTrue(ledger.recorded(SALE_ID));
        assertEq(cusd.balanceOf(agentOwner), CUSD_FEE);
        assertEq(cusd.balanceOf(buyer), 100 ether - CUSD_FEE);
    }

    function test_recordSale_emits_event() public {
        vm.prank(buyer);
        cusd.approve(address(ledger), CUSD_FEE);

        vm.expectEmit(true, true, true, true);
        emit SignalLedger.SaleRecorded(
            SALE_ID, agentId, buyer, SIG_HASH, address(cusd), 5e18, CUSD_FEE
        );
        vm.prank(buyer);
        ledger.recordSale(SALE_ID, agentId, SIG_HASH, address(cusd), 5e18);
    }

    function test_recordSale_rejects_duplicate_id() public {
        vm.prank(buyer);
        cusd.approve(address(ledger), 2 * CUSD_FEE);

        vm.prank(buyer);
        ledger.recordSale(SALE_ID, agentId, SIG_HASH, address(cusd), 5e18);

        vm.prank(buyer);
        vm.expectRevert(SignalLedger.DuplicateSaleId.selector);
        ledger.recordSale(SALE_ID, agentId, SIG_HASH, address(cusd), 5e18);
    }

    function test_recordSale_rejects_zero_sale_id() public {
        vm.prank(buyer);
        vm.expectRevert(SignalLedger.InvalidSaleId.selector);
        ledger.recordSale(bytes32(0), agentId, SIG_HASH, address(cusd), 5e18);
    }

    function test_recordSale_rejects_zero_signal_hash() public {
        vm.prank(buyer);
        vm.expectRevert(SignalLedger.InvalidSignalHash.selector);
        ledger.recordSale(SALE_ID, agentId, bytes32(0), address(cusd), 5e18);
    }

    function test_recordSale_rejects_disallowed_token() public {
        MockERC20 randomTok = new MockERC20("X", "X", 18);
        vm.prank(buyer);
        vm.expectRevert(SignalLedger.TokenNotAllowed.selector);
        ledger.recordSale(SALE_ID, agentId, SIG_HASH, address(randomTok), 5e18);
    }

    function test_recordSale_unknown_agent_reverts() public {
        vm.prank(buyer);
        cusd.approve(address(ledger), CUSD_FEE);
        vm.prank(buyer);
        vm.expectRevert(BeamRiderRegistry.UnknownAgent.selector);
        ledger.recordSale(SALE_ID, 9999, SIG_HASH, address(cusd), 5e18);
    }

    function test_recordSale_propagates_failed_transfer() public {
        vm.prank(buyer);
        cusd.approve(address(ledger), CUSD_FEE);
        cusd.setFailTransfers(true);

        vm.prank(buyer);
        vm.expectRevert(SignalLedger.TransferFailed.selector);
        ledger.recordSale(SALE_ID, agentId, SIG_HASH, address(cusd), 5e18);

        // The whole transaction reverts on `TransferFailed`, including the
        // `recorded[saleId] = true` write. The buyer can retry with the same
        // id once the transfer constraint is fixed.
        assertFalse(ledger.recorded(SALE_ID));
    }
}
