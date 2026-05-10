// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {Test} from "forge-std/Test.sol";
import {BeamRiderRegistry} from "../src/BeamRiderRegistry.sol";

contract BeamRiderRegistryTest is Test {
    BeamRiderRegistry internal reg;

    address internal alice = makeAddr("alice");
    address internal bob   = makeAddr("bob");
    bytes32 internal pkA   = keccak256("alice-pubkey");
    bytes32 internal pkB   = keccak256("bob-pubkey");

    function setUp() public {
        reg = new BeamRiderRegistry();
    }

    // -- registerAgent --------------------------------------------------------

    function test_register_assigns_sequential_ids_and_emits() public {
        vm.prank(alice);
        vm.expectEmit(true, true, false, true);
        emit BeamRiderRegistry.AgentRegistered(1, alice, pkA, "alpha", "https://alpha.example");
        uint256 id1 = reg.registerAgent(pkA, "alpha", "https://alpha.example");

        vm.prank(bob);
        uint256 id2 = reg.registerAgent(pkB, "bravo", "https://bravo.example");

        assertEq(id1, 1);
        assertEq(id2, 2);
        assertEq(reg.totalAgents(), 2);
        assertEq(reg.ownerOfAgent(1), alice);
        assertEq(reg.ownerOfAgent(2), bob);
        assertEq(reg.pubkeyOf(1), pkA);

        BeamRiderRegistry.Agent memory a = reg.agentOf(1);
        assertEq(a.owner, alice);
        assertEq(a.pubkey, pkA);
        assertEq(a.name, "alpha");
        assertEq(a.serviceUrl, "https://alpha.example");
    }

    function test_register_rejects_zero_pubkey() public {
        vm.expectRevert(BeamRiderRegistry.InvalidPubkey.selector);
        reg.registerAgent(bytes32(0), "alpha", "https://x");
    }

    function test_register_rejects_empty_name() public {
        vm.expectRevert(BeamRiderRegistry.InvalidName.selector);
        reg.registerAgent(pkA, "", "https://x");
    }

    function test_register_rejects_oversize_name() public {
        bytes memory big = new bytes(reg.MAX_NAME_BYTES() + 1);
        vm.expectRevert(BeamRiderRegistry.InvalidName.selector);
        reg.registerAgent(pkA, string(big), "https://x");
    }

    function test_register_rejects_empty_url() public {
        vm.expectRevert(BeamRiderRegistry.InvalidServiceUrl.selector);
        reg.registerAgent(pkA, "alpha", "");
    }

    function test_register_rejects_oversize_url() public {
        bytes memory big = new bytes(reg.MAX_URL_BYTES() + 1);
        vm.expectRevert(BeamRiderRegistry.InvalidServiceUrl.selector);
        reg.registerAgent(pkA, "alpha", string(big));
    }

    // -- updateMetadata -------------------------------------------------------

    function test_updateMetadata_owner_can_update() public {
        vm.prank(alice);
        uint256 id = reg.registerAgent(pkA, "alpha", "https://a");

        bytes32 newPk = keccak256("rotated");
        vm.prank(alice);
        reg.updateMetadata(id, newPk, "alpha2", "https://a2");

        BeamRiderRegistry.Agent memory a = reg.agentOf(id);
        assertEq(a.pubkey, newPk);
        assertEq(a.name, "alpha2");
        assertEq(a.serviceUrl, "https://a2");
    }

    function test_updateMetadata_non_owner_reverts() public {
        vm.prank(alice);
        uint256 id = reg.registerAgent(pkA, "alpha", "https://a");

        vm.prank(bob);
        vm.expectRevert(BeamRiderRegistry.NotAgentOwner.selector);
        reg.updateMetadata(id, pkB, "x", "https://x");
    }

    function test_updateMetadata_unknown_reverts() public {
        vm.expectRevert(BeamRiderRegistry.UnknownAgent.selector);
        reg.updateMetadata(99, pkA, "x", "https://x");
    }

    // -- transferAgentOwnership ----------------------------------------------

    function test_transfer_ownership_succeeds() public {
        vm.prank(alice);
        uint256 id = reg.registerAgent(pkA, "alpha", "https://a");

        vm.prank(alice);
        vm.expectEmit(true, true, true, false);
        emit BeamRiderRegistry.AgentOwnershipTransferred(id, alice, bob);
        reg.transferAgentOwnership(id, bob);

        assertEq(reg.ownerOfAgent(id), bob);
    }

    function test_transfer_ownership_zero_address_reverts() public {
        vm.prank(alice);
        uint256 id = reg.registerAgent(pkA, "alpha", "https://a");

        vm.prank(alice);
        vm.expectRevert(BeamRiderRegistry.InvalidNewOwner.selector);
        reg.transferAgentOwnership(id, address(0));
    }

    function test_transfer_ownership_non_owner_reverts() public {
        vm.prank(alice);
        uint256 id = reg.registerAgent(pkA, "alpha", "https://a");

        vm.prank(bob);
        vm.expectRevert(BeamRiderRegistry.NotAgentOwner.selector);
        reg.transferAgentOwnership(id, bob);
    }

    function test_unknown_agent_views_revert() public {
        vm.expectRevert(BeamRiderRegistry.UnknownAgent.selector);
        reg.agentOf(7);
        vm.expectRevert(BeamRiderRegistry.UnknownAgent.selector);
        reg.ownerOfAgent(7);
        vm.expectRevert(BeamRiderRegistry.UnknownAgent.selector);
        reg.pubkeyOf(7);
    }
}
