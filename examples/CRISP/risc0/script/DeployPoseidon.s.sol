// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "forge-std/console.sol";

/**
 * @notice Script to deploy the Poseidon library to a deterministic address
 * This script handles the setup for poseidon-solidity's proxy deployment pattern
 */
contract DeployPoseidonScript is Script {
    // You'll need to replace these with actual values from poseidon-solidity
    address constant POSEIDON_PROXY_ADDRESS = address(0); // Replace with actual address
    address constant POSEIDON_T3_ADDRESS = address(0); // Replace with actual address
    address constant PROXY_DEPLOYER = address(0); // Replace with actual address
    uint256 constant PROXY_GAS = 0; // Replace with actual gas amount
    bytes constant PROXY_TX = hex""; // Replace with actual transaction data
    bytes constant POSEIDON_T3_DATA = hex""; // Replace with actual bytecode

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");

        vm.startBroadcast(deployerPrivateKey);

        // Check if the proxy exists
        bytes memory proxyCode = vm.getCode(POSEIDON_PROXY_ADDRESS);
        if (keccak256(proxyCode) == keccak256(hex"")) {
            console.log("Deploying Poseidon proxy...");

            // Fund the keyless account
            (bool sent, ) = PROXY_DEPLOYER.call{value: PROXY_GAS}("");
            require(sent, "Failed to send Ether to proxy deployer");

            // Send the presigned transaction deploying the proxy
            // Note: This is a simplified version - in practice you might need
            // to use vm.broadcast in a specific way to handle raw transactions
            (bool success, ) = address(0).call(PROXY_TX);
            require(success, "Failed to deploy proxy");

            console.log("Proxy deployed to:", POSEIDON_PROXY_ADDRESS);
        } else {
            console.log(
                "Poseidon proxy already deployed at:",
                POSEIDON_PROXY_ADDRESS
            );
        }

        // Then deploy the hasher, if needed
        bytes memory poseidonCode = vm.getCode(POSEIDON_T3_ADDRESS);
        if (keccak256(poseidonCode) == keccak256(hex"")) {
            console.log("Deploying PoseidonT3...");

            (bool success, ) = POSEIDON_PROXY_ADDRESS.call(POSEIDON_T3_DATA);
            require(success, "Failed to deploy PoseidonT3");

            console.log("PoseidonT3 deployed to:", POSEIDON_T3_ADDRESS);
        } else {
            console.log("PoseidonT3 already deployed at:", POSEIDON_T3_ADDRESS);
        }

        vm.stopBroadcast();
    }
}
