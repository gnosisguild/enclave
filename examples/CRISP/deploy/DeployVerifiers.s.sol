// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script} from "forge-std/Script.sol";
import {console2} from "forge-std/console2.sol";
import {HonkVerifier} from "../contracts/CRISPVerifier.sol";
import {SemaphoreNoirVerifier} from "@semaphore-protocol/contracts-noir/base/SemaphoreNoirVerifier.sol";

/// @notice Deployment script for large verifier contracts
/// @dev This script deploys the complex verifier contracts separately to avoid compilation issues
contract DeployVerifiers is Script {
    function run() external {
        // Read and log the chainID
        uint256 chainId = block.chainid;
        console2.log("Deploying Verifiers on ChainID %d", chainId);

        // Get the private key from environment
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        vm.startBroadcast(deployerPrivateKey);

        // Deploy HonkVerifier
        console2.log("Deploying HonkVerifier...");
        HonkVerifier honkVerifier = new HonkVerifier();
        console2.log("Deployed HonkVerifier to", address(honkVerifier));

        // Deploy SemaphoreNoirVerifier
        console2.log("Deploying SemaphoreNoirVerifier...");
        SemaphoreNoirVerifier semaphoreNoirVerifier = new SemaphoreNoirVerifier();
        console2.log("Deployed SemaphoreNoirVerifier to", address(semaphoreNoirVerifier));

        vm.stopBroadcast();

        // Output addresses for use in main deployment
        console2.log("");
        console2.log("=== VERIFIER ADDRESSES ===");
        console2.log("HonkVerifier:", address(honkVerifier));
        console2.log("SemaphoreNoirVerifier:", address(semaphoreNoirVerifier));
        console2.log("=========================");
    }
} 