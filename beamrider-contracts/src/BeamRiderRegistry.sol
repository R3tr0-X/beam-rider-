// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

/// @title BeamRiderRegistry
/// @notice On-chain identity registry for BeamRider agents.
/// @dev Each agent occupies a 1-indexed `tokenId` mapping to
/// `(owner, ed25519PubKey, name, serviceUrl)`. This contract is intentionally
/// **not** ERC-721 compliant: there are no transfers via `safeTransferFrom`,
/// no allowances, no metadata URI extension. The data shape is what the Rust
/// backend reads; nothing more.
contract BeamRiderRegistry {
    struct Agent {
        address owner;
        bytes32 pubkey;       // Ed25519 public key, 32 bytes
        string name;
        string serviceUrl;
    }

    /// @notice Cap on agent display name length, in bytes.
    uint256 public constant MAX_NAME_BYTES = 64;

    /// @notice Cap on service-URL length, in bytes.
    uint256 public constant MAX_URL_BYTES = 256;

    /// @notice Total agents ever registered. Doubles as the high-water mark
    /// for `tokenId` allocation; ids start at 1.
    uint256 public totalAgents;

    mapping(uint256 => Agent) private _agents;

    error InvalidPubkey();
    error InvalidName();
    error InvalidServiceUrl();
    error UnknownAgent();
    error NotAgentOwner();
    error InvalidNewOwner();

    event AgentRegistered(
        uint256 indexed tokenId,
        address indexed owner,
        bytes32 pubkey,
        string name,
        string serviceUrl
    );
    event AgentMetadataUpdated(
        uint256 indexed tokenId,
        bytes32 pubkey,
        string name,
        string serviceUrl
    );
    event AgentOwnershipTransferred(
        uint256 indexed tokenId,
        address indexed previousOwner,
        address indexed newOwner
    );

    function registerAgent(
        bytes32 pubkey,
        string calldata name,
        string calldata serviceUrl
    ) external returns (uint256 tokenId) {
        _validateMetadata(pubkey, name, serviceUrl);
        unchecked { tokenId = ++totalAgents; }
        _agents[tokenId] = Agent({
            owner: msg.sender,
            pubkey: pubkey,
            name: name,
            serviceUrl: serviceUrl
        });
        emit AgentRegistered(tokenId, msg.sender, pubkey, name, serviceUrl);
    }

    function updateMetadata(
        uint256 tokenId,
        bytes32 pubkey,
        string calldata name,
        string calldata serviceUrl
    ) external {
        Agent storage a = _agents[tokenId];
        if (a.owner == address(0)) revert UnknownAgent();
        if (a.owner != msg.sender) revert NotAgentOwner();
        _validateMetadata(pubkey, name, serviceUrl);
        a.pubkey = pubkey;
        a.name = name;
        a.serviceUrl = serviceUrl;
        emit AgentMetadataUpdated(tokenId, pubkey, name, serviceUrl);
    }

    function transferAgentOwnership(uint256 tokenId, address newOwner) external {
        Agent storage a = _agents[tokenId];
        address current = a.owner;
        if (current == address(0)) revert UnknownAgent();
        if (current != msg.sender) revert NotAgentOwner();
        if (newOwner == address(0)) revert InvalidNewOwner();
        a.owner = newOwner;
        emit AgentOwnershipTransferred(tokenId, current, newOwner);
    }

    function agentOf(uint256 tokenId) external view returns (Agent memory) {
        Agent memory a = _agents[tokenId];
        if (a.owner == address(0)) revert UnknownAgent();
        return a;
    }

    function ownerOfAgent(uint256 tokenId) external view returns (address) {
        address o = _agents[tokenId].owner;
        if (o == address(0)) revert UnknownAgent();
        return o;
    }

    function pubkeyOf(uint256 tokenId) external view returns (bytes32) {
        Agent storage a = _agents[tokenId];
        if (a.owner == address(0)) revert UnknownAgent();
        return a.pubkey;
    }

    function _validateMetadata(
        bytes32 pubkey,
        string calldata name,
        string calldata serviceUrl
    ) private pure {
        if (pubkey == bytes32(0)) revert InvalidPubkey();
        uint256 nameLen = bytes(name).length;
        if (nameLen == 0 || nameLen > MAX_NAME_BYTES) revert InvalidName();
        uint256 urlLen = bytes(serviceUrl).length;
        if (urlLen == 0 || urlLen > MAX_URL_BYTES) revert InvalidServiceUrl();
    }
}
