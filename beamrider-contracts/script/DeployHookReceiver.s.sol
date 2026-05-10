// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {Script, console2} from "forge-std/Script.sol";
import {HookReceiver} from "../src/HookReceiver.sol";

/// @notice Deploys `HookReceiver` to a CCTP V2 destination chain.
/// @dev Required env vars:
///   - `DEPLOYER_PRIVATE_KEY`
///   - `MESSAGE_TRANSMITTER_V2`  destination-chain MessageTransmitterV2
///   - `EXPECTED_SOURCE_DOMAIN`  source CCTP domain id (uint32)
///   - `EXPECTED_SENDER`         source caller as bytes32 (0x-padded)
///   - `USDC_ADDRESS`            destination USDC
///   - `AAVE_V3_POOL`            destination Aave V3 pool
/// Optional: `BEAMRIDER_ADMIN` (defaults to deployer).
///
/// Mainnet CCTP V2 (deterministic on Base, Arbitrum, etc):
///   MessageTransmitterV2 = 0x81D40F21F12A8F0E3252Bccb954D722d4c464B64
///   TokenMessengerV2     = 0x28b5a0e9C621a5BadaA536219b3a228C8168cf5d
contract DeployHookReceiver is Script {
    function run() external returns (HookReceiver hookReceiver) {
        uint256 pk = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(pk);

        address messageTransmitter = vm.envAddress("MESSAGE_TRANSMITTER_V2");
        uint32 srcDomain = uint32(vm.envUint("EXPECTED_SOURCE_DOMAIN"));
        bytes32 srcSender = vm.envBytes32("EXPECTED_SENDER");
        address usdc = vm.envAddress("USDC_ADDRESS");
        address aavePool = vm.envAddress("AAVE_V3_POOL");
        address admin = _envAddressOr("BEAMRIDER_ADMIN", deployer);

        vm.startBroadcast(pk);
        hookReceiver = new HookReceiver(
            messageTransmitter,
            srcDomain,
            srcSender,
            usdc,
            aavePool,
            admin
        );
        vm.stopBroadcast();

        console2.log("HookReceiver:        ", address(hookReceiver));
        console2.log("MessageTransmitterV2:", messageTransmitter);
        console2.log("ExpectedSrcDomain:   ", srcDomain);
        console2.logBytes32(srcSender);
        console2.log("USDC:                ", usdc);
        console2.log("AaveV3Pool:          ", aavePool);
        console2.log("Admin:               ", admin);
    }

    function _envAddressOr(string memory key, address fallback_) private view returns (address) {
        try vm.envAddress(key) returns (address v) {
            return v;
        } catch {
            return fallback_;
        }
    }
}
