// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {IERC20} from "./interfaces/IERC20.sol";
import {Ownable} from "./utils/Ownable.sol";

/// @notice Narrow view of `BeamRiderRegistry` used by the ledger.
interface IBeamRiderRegistryView {
    function ownerOfAgent(uint256 tokenId) external view returns (address);
}

/// @title SignalLedger
/// @notice Records x402 signal sales as Celo events.
/// @dev The buyer pays a small recording fee in an allow-listed ERC-20 (cUSD or
/// USDC). The fee is forwarded to the agent's owner — this is what gives the
/// buyer an incentive to record the sale on-chain (proof-of-receipt) and what
/// accumulates the Celo transaction count BeamRider's KarmaGAP milestones
/// point at. `paidAmount` is the off-chain x402 payment value, logged but not
/// transferred.
contract SignalLedger is Ownable {
    IBeamRiderRegistryView public immutable registry;

    /// @notice `0` means the token is not allow-listed.
    mapping(address => uint256) public minFee;

    /// @notice Replay-protection set on the unique `saleId`.
    mapping(bytes32 => bool) public recorded;

    error InvalidRegistry();
    error InvalidSaleId();
    error InvalidSignalHash();
    error DuplicateSaleId();
    error TokenNotAllowed();
    error TransferFailed();

    event TokenAllowed(address indexed token, uint256 minFee);
    event TokenDisallowed(address indexed token);
    event SaleRecorded(
        bytes32 indexed saleId,
        uint256 indexed agentTokenId,
        address indexed buyer,
        bytes32 signalHash,
        address feeToken,
        uint256 paidAmount,
        uint256 feeCharged
    );

    constructor(address registry_, address initialOwner) Ownable(initialOwner) {
        if (registry_ == address(0)) revert InvalidRegistry();
        registry = IBeamRiderRegistryView(registry_);
    }

    function setAllowedToken(address token, uint256 fee) external onlyOwner {
        if (fee == 0) {
            delete minFee[token];
            emit TokenDisallowed(token);
        } else {
            minFee[token] = fee;
            emit TokenAllowed(token, fee);
        }
    }

    /// @notice Record a sale. The agent's owner is paid `minFee[feeToken]`
    /// directly from `msg.sender`.
    /// @param saleId      Caller-chosen unique identifier (e.g. x402 receipt id).
    /// @param agentTokenId The BeamRider agent registry id.
    /// @param signalHash  Off-chain hash of the signal payload (canonical bytes).
    /// @param feeToken    Allow-listed ERC-20 used to pay the recording fee.
    /// @param paidAmount  Off-chain x402 payment amount, logged for analytics.
    function recordSale(
        bytes32 saleId,
        uint256 agentTokenId,
        bytes32 signalHash,
        address feeToken,
        uint256 paidAmount
    ) external {
        if (saleId == bytes32(0)) revert InvalidSaleId();
        if (signalHash == bytes32(0)) revert InvalidSignalHash();
        if (recorded[saleId]) revert DuplicateSaleId();

        uint256 fee = minFee[feeToken];
        if (fee == 0) revert TokenNotAllowed();

        // `ownerOfAgent` reverts on unknown id; surface that revert directly.
        address agentOwner = registry.ownerOfAgent(agentTokenId);

        // Effects
        recorded[saleId] = true;

        // Interactions
        bool ok = IERC20(feeToken).transferFrom(msg.sender, agentOwner, fee);
        if (!ok) revert TransferFailed();

        emit SaleRecorded(
            saleId,
            agentTokenId,
            msg.sender,
            signalHash,
            feeToken,
            paidAmount,
            fee
        );
    }
}
