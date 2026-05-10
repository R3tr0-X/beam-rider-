// SPDX-License-Identifier: MIT
pragma solidity ^0.8.25;

import {Script, console2} from "forge-std/Script.sol";
import {BeamRiderRegistry} from "../src/BeamRiderRegistry.sol";
import {SignalLedger} from "../src/SignalLedger.sol";
import {YieldStrategy} from "../src/YieldStrategy.sol";

/// @notice Deploys the three Celo-side BeamRider contracts.
/// @dev Reads `DEPLOYER_PRIVATE_KEY` from env. Optional env vars:
///   - `BEAMRIDER_ADMIN`     fallback owner for ledger/strategy (default: deployer)
///   - `CUSD_ADDRESS`        if set + non-zero, allow-list with `CUSD_FEE`
///   - `CUSD_FEE`            min fee in cUSD atoms (default 1e18)
///   - `USDC_ADDRESS`        if set + non-zero, allow-list with `USDC_FEE`
///   - `USDC_FEE`            min fee in USDC atoms (default 1e5)
contract DeployCelo is Script {
    function run()
        external
        returns (
            BeamRiderRegistry registry,
            SignalLedger ledger,
            YieldStrategy strategy
        )
    {
        uint256 pk = vm.envUint("DEPLOYER_PRIVATE_KEY");
        address deployer = vm.addr(pk);
        address admin = _envAddressOr("BEAMRIDER_ADMIN", deployer);

        vm.startBroadcast(pk);

        registry = new BeamRiderRegistry();
        ledger   = new SignalLedger(address(registry), admin);
        strategy = new YieldStrategy(admin);

        address cusd = _envAddressOr("CUSD_ADDRESS", address(0));
        if (cusd != address(0)) {
            uint256 fee = _envUintOr("CUSD_FEE", 1e18);
            ledger.setAllowedToken(cusd, fee);
            strategy.setApprovedToken(cusd, true);
        }
        address usdc = _envAddressOr("USDC_ADDRESS", address(0));
        if (usdc != address(0)) {
            uint256 fee = _envUintOr("USDC_FEE", 1e5);
            ledger.setAllowedToken(usdc, fee);
            strategy.setApprovedToken(usdc, true);
        }

        vm.stopBroadcast();

        console2.log("BeamRiderRegistry:", address(registry));
        console2.log("SignalLedger:     ", address(ledger));
        console2.log("YieldStrategy:    ", address(strategy));
        console2.log("Admin:            ", admin);
    }

    function _envAddressOr(string memory key, address fallback_) private view returns (address) {
        try vm.envAddress(key) returns (address v) {
            return v;
        } catch {
            return fallback_;
        }
    }

    function _envUintOr(string memory key, uint256 fallback_) private view returns (uint256) {
        try vm.envUint(key) returns (uint256 v) {
            return v;
        } catch {
            return fallback_;
        }
    }
}
