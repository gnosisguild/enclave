// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script} from "forge-std/Script.sol";
import {console2} from "forge-std/console2.sol";
import {HonkVerifier} from "../contracts/CRISPVerifier.sol";

/// @notice Standalone deployment script for the HonkVerifier contract
/// @dev This script deploys only the HonkVerifier to test compilation and deployment
contract DeployHonkVerifier is Script {
    function run() external {
        // Read and log the chainID
        uint256 chainId = block.chainid;
        console2.log("Deploying HonkVerifier on ChainID %d", chainId);

        // Get the private key from environment
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        vm.startBroadcast(deployerPrivateKey);

        // Deploy HonkVerifier
        console2.log("Deploying HonkVerifier...");
        HonkVerifier honkVerifier = new HonkVerifier();
        console2.log("Deployed HonkVerifier to", address(honkVerifier));

        vm.stopBroadcast();
    }
} 