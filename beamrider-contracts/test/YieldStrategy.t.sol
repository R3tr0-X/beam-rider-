// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {Test} from "forge-std/Test.sol";
import {YieldStrategy} from "../src/YieldStrategy.sol";
import {MockERC20} from "./mocks/MockERC20.sol";

contract YieldStrategyTest is Test {
    YieldStrategy internal strat;
    MockERC20 internal usdc;

    address internal admin   = makeAddr("admin");
    address internal vault   = makeAddr("aaveCeloVault");
    address internal bridge  = makeAddr("squidRouter");
    address internal sweep   = makeAddr("sweep");
    address internal stranger = makeAddr("stranger");

    uint256 internal constant DEPOSIT = 1_000 * 1e6;
    bytes32 internal constant COMMIT  = keccak256("commit-v1");

    function setUp() public {
        strat = new YieldStrategy(admin);
        usdc = new MockERC20("USD Coin", "USDC", 6);

        vm.prank(admin);
        strat.setApprovedToken(address(usdc), true);

        usdc.mint(address(strat), DEPOSIT * 5);
    }

    // -- propose --------------------------------------------------------------

    function test_propose_assigns_sequential_ids_and_emits() public {
        vm.expectEmit(true, true, false, true);
        emit YieldStrategy.StrategyProposed(1, YieldStrategy.Venue.AaveCelo, 42220, DEPOSIT, COMMIT);
        vm.prank(admin);
        uint256 id = strat.proposeStrategy(YieldStrategy.Venue.AaveCelo, 42220, DEPOSIT, COMMIT);

        assertEq(id, 1);
        assertEq(strat.totalProposals(), 1);

        YieldStrategy.Proposal memory p = strat.proposalOf(id);
        assertEq(uint256(p.venue), uint256(YieldStrategy.Venue.AaveCelo));
        assertEq(p.destChain, 42220);
        assertEq(p.amount, DEPOSIT);
        assertEq(p.commitHash, COMMIT);
        assertEq(uint256(p.status), uint256(YieldStrategy.ProposalStatus.Proposed));
    }

    function test_propose_only_owner() public {
        vm.expectRevert();
        strat.proposeStrategy(YieldStrategy.Venue.AaveCelo, 42220, DEPOSIT, COMMIT);
    }

    function test_propose_rejects_unknown_venue() public {
        vm.prank(admin);
        vm.expectRevert(YieldStrategy.InvalidVenue.selector);
        strat.proposeStrategy(YieldStrategy.Venue.Unknown, 42220, DEPOSIT, COMMIT);
    }

    function test_propose_rejects_zero_amount() public {
        vm.prank(admin);
        vm.expectRevert(YieldStrategy.InvalidAmount.selector);
        strat.proposeStrategy(YieldStrategy.Venue.AaveCelo, 42220, 0, COMMIT);
    }

    // -- executeStrategyOnCelo ------------------------------------------------

    function test_executeOnCelo_transfers_to_vault() public {
        uint256 id = _propose(YieldStrategy.Venue.MoolaCelo, 42220);

        vm.expectEmit(true, true, true, true);
        emit YieldStrategy.StrategyExecutedOnCelo(id, vault, address(usdc), DEPOSIT);
        vm.prank(admin);
        strat.executeStrategyOnCelo(id, vault, address(usdc));

        assertEq(usdc.balanceOf(vault), DEPOSIT);
        assertEq(uint256(strat.proposalOf(id).status), uint256(YieldStrategy.ProposalStatus.ExecutedOnCelo));
    }

    function test_executeOnCelo_reverts_for_cross_chain_venue() public {
        uint256 id = _propose(YieldStrategy.Venue.AaveArbitrum, 42161);
        vm.prank(admin);
        vm.expectRevert(YieldStrategy.VenueNotOnCelo.selector);
        strat.executeStrategyOnCelo(id, vault, address(usdc));
    }

    function test_executeOnCelo_reverts_zero_vault() public {
        uint256 id = _propose(YieldStrategy.Venue.AaveCelo, 42220);
        vm.prank(admin);
        vm.expectRevert(YieldStrategy.InvalidVault.selector);
        strat.executeStrategyOnCelo(id, address(0), address(usdc));
    }

    function test_executeOnCelo_reverts_unapproved_token() public {
        MockERC20 other = new MockERC20("Other", "OTH", 18);
        uint256 id = _propose(YieldStrategy.Venue.AaveCelo, 42220);
        vm.prank(admin);
        vm.expectRevert(YieldStrategy.TokenNotApproved.selector);
        strat.executeStrategyOnCelo(id, vault, address(other));
    }

    function test_executeOnCelo_reverts_double_execute() public {
        uint256 id = _propose(YieldStrategy.Venue.AaveCelo, 42220);
        vm.prank(admin);
        strat.executeStrategyOnCelo(id, vault, address(usdc));

        vm.prank(admin);
        vm.expectRevert(YieldStrategy.ProposalNotPending.selector);
        strat.executeStrategyOnCelo(id, vault, address(usdc));
    }

    // -- executeStrategyCrossChain --------------------------------------------

    function test_executeCrossChain_transfers_to_bridge() public {
        uint256 id = _propose(YieldStrategy.Venue.AaveArbitrum, 42161);

        vm.expectEmit(true, true, true, true);
        emit YieldStrategy.BridgeInitiated(id, bridge, address(usdc), DEPOSIT);
        vm.prank(admin);
        strat.executeStrategyCrossChain(id, bridge, address(usdc));

        assertEq(usdc.balanceOf(bridge), DEPOSIT);
        assertEq(uint256(strat.proposalOf(id).status), uint256(YieldStrategy.ProposalStatus.ExecutedCrossChain));
    }

    function test_executeCrossChain_reverts_for_celo_venue() public {
        uint256 id = _propose(YieldStrategy.Venue.AaveCelo, 42220);
        vm.prank(admin);
        vm.expectRevert(YieldStrategy.VenueNotCrossChain.selector);
        strat.executeStrategyCrossChain(id, bridge, address(usdc));
    }

    function test_executeCrossChain_reverts_zero_bridge() public {
        uint256 id = _propose(YieldStrategy.Venue.AaveArbitrum, 42161);
        vm.prank(admin);
        vm.expectRevert(YieldStrategy.InvalidBridge.selector);
        strat.executeStrategyCrossChain(id, address(0), address(usdc));
    }

    // -- cancel ---------------------------------------------------------------

    function test_cancel_marks_status() public {
        uint256 id = _propose(YieldStrategy.Venue.AaveCelo, 42220);
        vm.prank(admin);
        strat.cancelProposal(id);
        assertEq(uint256(strat.proposalOf(id).status), uint256(YieldStrategy.ProposalStatus.Cancelled));
    }

    function test_cancel_reverts_when_already_executed() public {
        uint256 id = _propose(YieldStrategy.Venue.AaveCelo, 42220);
        vm.prank(admin);
        strat.executeStrategyOnCelo(id, vault, address(usdc));
        vm.prank(admin);
        vm.expectRevert(YieldStrategy.ProposalNotPending.selector);
        strat.cancelProposal(id);
    }

    function test_cancel_unknown_reverts() public {
        vm.prank(admin);
        vm.expectRevert(YieldStrategy.UnknownProposal.selector);
        strat.cancelProposal(123);
    }

    // -- withdraw -------------------------------------------------------------

    function test_withdraw_transfers_balance() public {
        vm.prank(admin);
        strat.withdraw(address(usdc), sweep, DEPOSIT);
        assertEq(usdc.balanceOf(sweep), DEPOSIT);
    }

    function test_withdraw_only_owner() public {
        vm.expectRevert();
        strat.withdraw(address(usdc), sweep, DEPOSIT);
    }

    function test_withdraw_zero_to_reverts() public {
        vm.prank(admin);
        vm.expectRevert(YieldStrategy.InvalidVault.selector);
        strat.withdraw(address(usdc), address(0), DEPOSIT);
    }

    // -- helpers --------------------------------------------------------------

    function _propose(YieldStrategy.Venue v, uint32 destChain) private returns (uint256 id) {
        vm.prank(admin);
        id = strat.proposeStrategy(v, destChain, DEPOSIT, COMMIT);
    }
}
