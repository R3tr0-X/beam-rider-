// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {IERC20} from "./interfaces/IERC20.sol";
import {Ownable} from "./utils/Ownable.sol";

/// @title YieldStrategy
/// @notice BeamRider's on-Celo treasury and decision log.
/// @dev The off-chain agent commits a strategy here before executing it. Each
/// proposal is one Celo transaction (POS-countable). Execution either deposits
/// into an on-Celo venue directly, or releases ERC-20 custody to a configured
/// bridge router which then carries the funds to a CCTP domain off-chain.
///
/// The bridge call itself is **not** made from this contract: that surface is
/// untyped, hard to test, and an arbitrary-call hole that the agent operator
/// (the only owner) does not need.
contract YieldStrategy is Ownable {
    /// @notice Mirror of `beamrider-backend/src/domain/strategy.rs::Venue`.
    /// `Unknown` is the zero value; valid venues are 1..=5.
    enum Venue {
        Unknown,
        AaveCelo,
        MoolaCelo,
        UbeswapLp,
        AaveArbitrum,
        AaveBase
    }

    enum ProposalStatus {
        Proposed,
        ExecutedOnCelo,
        ExecutedCrossChain,
        Cancelled
    }

    struct Proposal {
        Venue venue;
        uint32 destChain;       // EIP-155 chain id of the deposit destination
        uint256 amount;         // token atoms (decimals-native)
        bytes32 commitHash;     // off-chain agent's pre-execution commit
        ProposalStatus status;
        uint256 proposedAt;     // block.timestamp; doubles as the existence flag
    }

    uint256 public totalProposals;
    mapping(uint256 => Proposal) private _proposals;
    mapping(address => bool) public approvedToken;

    error InvalidVenue();
    error InvalidAmount();
    error UnknownProposal();
    error ProposalNotPending();
    error TokenNotApproved();
    error InvalidVault();
    error InvalidBridge();
    error TransferFailed();
    error VenueNotOnCelo();
    error VenueNotCrossChain();

    event TokenApprovalSet(address indexed token, bool approved);
    event StrategyProposed(
        uint256 indexed proposalId,
        Venue indexed venue,
        uint32 destChain,
        uint256 amount,
        bytes32 commitHash
    );
    event StrategyExecutedOnCelo(
        uint256 indexed proposalId,
        address indexed vault,
        address indexed token,
        uint256 amount
    );
    event BridgeInitiated(
        uint256 indexed proposalId,
        address indexed bridge,
        address indexed token,
        uint256 amount
    );
    event ProposalCancelled(uint256 indexed proposalId);
    event Withdrawn(address indexed token, address indexed to, uint256 amount);

    constructor(address initialOwner) Ownable(initialOwner) {}

    function setApprovedToken(address token, bool approved) external onlyOwner {
        approvedToken[token] = approved;
        emit TokenApprovalSet(token, approved);
    }

    function proposeStrategy(
        Venue venue,
        uint32 destChain,
        uint256 amount,
        bytes32 commitHash
    ) external onlyOwner returns (uint256 id) {
        if (venue == Venue.Unknown) revert InvalidVenue();
        if (amount == 0) revert InvalidAmount();

        unchecked { id = ++totalProposals; }
        _proposals[id] = Proposal({
            venue: venue,
            destChain: destChain,
            amount: amount,
            commitHash: commitHash,
            status: ProposalStatus.Proposed,
            proposedAt: block.timestamp
        });
        emit StrategyProposed(id, venue, destChain, amount, commitHash);
    }

    function cancelProposal(uint256 id) external onlyOwner {
        Proposal storage p = _proposals[id];
        if (p.proposedAt == 0) revert UnknownProposal();
        if (p.status != ProposalStatus.Proposed) revert ProposalNotPending();
        p.status = ProposalStatus.Cancelled;
        emit ProposalCancelled(id);
    }

    /// @notice Stay-on-Celo execution: transfer the proposed amount to the
    /// on-Celo `vault` (Aave V3 Celo, Moola, or an Ubeswap LP shim).
    function executeStrategyOnCelo(
        uint256 id,
        address vault,
        address token
    ) external onlyOwner {
        Proposal storage p = _proposals[id];
        if (p.proposedAt == 0) revert UnknownProposal();
        if (p.status != ProposalStatus.Proposed) revert ProposalNotPending();
        if (vault == address(0)) revert InvalidVault();
        if (!approvedToken[token]) revert TokenNotApproved();
        if (!_isOnCeloVenue(p.venue)) revert VenueNotOnCelo();

        uint256 amount = p.amount;
        p.status = ProposalStatus.ExecutedOnCelo;

        bool ok = IERC20(token).transfer(vault, amount);
        if (!ok) revert TransferFailed();
        emit StrategyExecutedOnCelo(id, vault, token, amount);
    }

    /// @notice Cross-chain execution: release token custody to the bridge
    /// router (Squid / Across). The router pulls funds via its own logic
    /// off-chain; this contract only emits the audit event.
    function executeStrategyCrossChain(
        uint256 id,
        address bridge,
        address token
    ) external onlyOwner {
        Proposal storage p = _proposals[id];
        if (p.proposedAt == 0) revert UnknownProposal();
        if (p.status != ProposalStatus.Proposed) revert ProposalNotPending();
        if (bridge == address(0)) revert InvalidBridge();
        if (!approvedToken[token]) revert TokenNotApproved();
        if (_isOnCeloVenue(p.venue)) revert VenueNotCrossChain();

        uint256 amount = p.amount;
        p.status = ProposalStatus.ExecutedCrossChain;

        bool ok = IERC20(token).transfer(bridge, amount);
        if (!ok) revert TransferFailed();
        emit BridgeInitiated(id, bridge, token, amount);
    }

    function withdraw(address token, address to, uint256 amount) external onlyOwner {
        if (to == address(0)) revert InvalidVault();
        bool ok = IERC20(token).transfer(to, amount);
        if (!ok) revert TransferFailed();
        emit Withdrawn(token, to, amount);
    }

    function proposalOf(uint256 id) external view returns (Proposal memory) {
        Proposal memory p = _proposals[id];
        if (p.proposedAt == 0) revert UnknownProposal();
        return p;
    }

    function _isOnCeloVenue(Venue v) private pure returns (bool) {
        return v == Venue.AaveCelo || v == Venue.MoolaCelo || v == Venue.UbeswapLp;
    }
}
