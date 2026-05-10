// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {IERC20} from "./interfaces/IERC20.sol";
import {IAaveV3Pool} from "./interfaces/IAaveV3Pool.sol";
import {IMessageHandlerV2} from "./interfaces/IMessageHandlerV2.sol";
import {HookDataCodec} from "./libraries/HookDataCodec.sol";
import {Ownable} from "./utils/Ownable.sol";

/// @title HookReceiver
/// @notice Destination-chain (Base / Arbitrum) terminus for BeamRider's CCTP
/// V2 burn-with-hook rebalances.
/// @dev On a finalized message, decodes the BeamRider hookData (see
/// `HookDataCodec`) and either supplies the freshly-minted USDC into Aave V3
/// or transfers it to a return address. Unfinalized messages are rejected: a
/// rebalance that has not finalized is a rebalance we will not commit to.
///
/// Security invariants enforced on every callback:
///   1. `msg.sender == messageTransmitter` — only the local CCTP transmitter
///      may invoke this contract. Without this, anyone could airdrop dust and
///      forge a `hookData` payload.
///   2. `sourceDomain == expectedSourceDomain` — pinned to the CCTP domain id
///      of the source chain (e.g. Base when receiving on Arbitrum).
///   3. `sender == expectedSender` — pinned to the BeamRider source-side
///      `TokenMessengerV2` caller (the bytes32 form of the agent's bridge
///      contract on the source chain).
contract HookReceiver is IMessageHandlerV2, Ownable {
    using HookDataCodec for bytes;

    address public immutable messageTransmitter;
    uint32  public immutable expectedSourceDomain;
    bytes32 public immutable expectedSender;
    IERC20  public immutable usdc;
    IAaveV3Pool public immutable aavePool;

    error UnauthorizedTransmitter();
    error UnexpectedSourceDomain();
    error UnexpectedSender();
    error UnsupportedAction();
    error UnfinalizedNotSupported();
    error InvalidConstructorArg();
    error InvalidVault();
    error TransferFailed();

    event Deposited(uint8 indexed action, address indexed vault, uint256 amount, bytes metadata);
    event ReturnedHome(address indexed destination, uint256 amount, bytes metadata);
    event TokenRescued(address indexed token, address indexed to, uint256 amount);

    constructor(
        address messageTransmitter_,
        uint32 expectedSourceDomain_,
        bytes32 expectedSender_,
        address usdc_,
        address aavePool_,
        address initialOwner
    ) Ownable(initialOwner) {
        if (
            messageTransmitter_ == address(0) ||
            usdc_ == address(0) ||
            aavePool_ == address(0) ||
            expectedSender_ == bytes32(0)
        ) revert InvalidConstructorArg();
        messageTransmitter = messageTransmitter_;
        expectedSourceDomain = expectedSourceDomain_;
        expectedSender = expectedSender_;
        usdc = IERC20(usdc_);
        aavePool = IAaveV3Pool(aavePool_);
    }

    /// @inheritdoc IMessageHandlerV2
    function handleReceiveFinalizedMessage(
        uint32 sourceDomain,
        bytes32 sender,
        uint256 /* finalityThresholdExecuted */,
        bytes calldata messageBody
    ) external returns (bool) {
        _authenticate(sourceDomain, sender);

        (uint8 action, address vault, bytes calldata metadata) =
            HookDataCodec.decode(messageBody);
        if (vault == address(0)) revert InvalidVault();

        uint256 amount = usdc.balanceOf(address(this));
        // A zero-balance call is benign (no funds to act on) but indicates a
        // misordered relay or a duplicate replay; surface it as the codec
        // sees it.
        if (amount == 0) revert TransferFailed();

        if (action == HookDataCodec.ACTION_DEPOSIT_AAVE) {
            // Approve exactly `amount` and supply on behalf of `vault`.
            // USDC and cUSD are both standard ERC-20s; their `approve` cannot
            // fail, so a single call is sufficient.
            usdc.approve(address(aavePool), amount);
            aavePool.supply(address(usdc), amount, vault, 0);
            emit Deposited(action, vault, amount, metadata);
        } else if (action == HookDataCodec.ACTION_RETURN_HOME) {
            bool ok = usdc.transfer(vault, amount);
            if (!ok) revert TransferFailed();
            emit ReturnedHome(vault, amount, metadata);
        } else {
            revert UnsupportedAction();
        }
        return true;
    }

    /// @inheritdoc IMessageHandlerV2
    function handleReceiveUnfinalizedMessage(
        uint32 /* sourceDomain */,
        bytes32 /* sender */,
        uint256 /* finalityThresholdExecuted */,
        bytes calldata /* messageBody */
    ) external pure returns (bool) {
        revert UnfinalizedNotSupported();
    }

    /// @notice Sweep an arbitrary ERC-20 stuck on this contract.
    /// @dev Owner-restricted. Intended for unrecognized airdrops or for funds
    /// stranded by a mis-encoded hook payload that the codec rejected.
    function rescueToken(address token, address to, uint256 amount) external onlyOwner {
        if (to == address(0)) revert InvalidVault();
        bool ok = IERC20(token).transfer(to, amount);
        if (!ok) revert TransferFailed();
        emit TokenRescued(token, to, amount);
    }

    function _authenticate(uint32 sourceDomain, bytes32 sender) private view {
        if (msg.sender != messageTransmitter) revert UnauthorizedTransmitter();
        if (sourceDomain != expectedSourceDomain) revert UnexpectedSourceDomain();
        if (sender != expectedSender) revert UnexpectedSender();
    }
}
