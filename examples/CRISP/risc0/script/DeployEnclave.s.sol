// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import "forge-std/Script.sol";
import "forge-std/console.sol";
import "../contracts/enclave/Enclave.sol";
import "../contracts/enclave/registry/CiphernodeRegistryOwnable.sol";
import "../contracts/enclave/registry/NaiveRegistryFilter.sol";
import "poseidon-solidity/PoseidonT3.sol";

contract DeployEnclaveScript is Script {
    // Constants
    uint256 constant THIRTY_DAYS_IN_SECONDS = 60 * 60 * 24 * 30;
    address constant ADDRESS_ONE = 0x0000000000000000000000000000000000000001;

    // Poseidon proxy details - will need to be adjusted for your specific Poseidon implementation
    address immutable POSEIDON_PROXY_ADDRESS;
    address immutable POSEIDON_T3_ADDRESS;
    bytes POSEIDON_T3_DATA;
    address immutable PROXY_DEPLOYER;
    uint256 immutable PROXY_GAS;
    bytes PROXY_TX;

    constructor() {
        // These would be provided by your poseidon-solidity library
        // You'll need to replace these with the actual values from your library
        POSEIDON_PROXY_ADDRESS = 0x0000000000000000000000000000000000000000; // Replace with actual proxy address
        POSEIDON_T3_ADDRESS = 0x0000000000000000000000000000000000000000; // Replace with actual PoseidonT3 address
        PROXY_DEPLOYER = 0x0000000000000000000000000000000000000000; // Replace with actual proxy deployer
        PROXY_GAS = 0; // Replace with actual gas value

        // These would be actual bytecode and transaction data
        POSEIDON_T3_DATA = hex""; // Replace with actual PoseidonT3 data
        PROXY_TX = hex""; // Replace with actual proxy transaction
    }

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address deployer = vm.addr(deployerPrivateKey);

        vm.startBroadcast(deployerPrivateKey);

        // Deploy Poseidon proxy if it doesn't exist
        bytes memory proxyCode = vm.getCode(POSEIDON_PROXY_ADDRESS);
        if (keccak256(proxyCode) == keccak256(hex"")) {
            console.log("Deploying Poseidon proxy...");

            // Fund the keyless account
            (bool sent, ) = PROXY_DEPLOYER.call{value: PROXY_GAS}("");
            require(sent, "Failed to send Ether to proxy deployer");

            // Send the presigned transaction
            vm.broadcast(0); // Use a special broadcast that allows sending raw transactions
            (bool success, ) = address(0).call(PROXY_TX);
            require(success, "Failed to deploy proxy");

            console.log("Proxy deployed to:", POSEIDON_PROXY_ADDRESS);
        }

        // Deploy PoseidonT3 if needed
        bytes memory poseidonCode = vm.getCode(POSEIDON_T3_ADDRESS);
        if (keccak256(poseidonCode) == keccak256(hex"")) {
            console.log("Deploying PoseidonT3...");

            (bool success, ) = POSEIDON_PROXY_ADDRESS.call(POSEIDON_T3_DATA);
            require(success, "Failed to deploy PoseidonT3");

            console.log("PoseidonT3 deployed to:", POSEIDON_T3_ADDRESS);
        }

        // Deploy Enclave contract
        console.log("Deploying Enclave contract...");
        Enclave enclave = new Enclave(
            deployer,
            ADDRESS_ONE,
            THIRTY_DAYS_IN_SECONDS
        );
        console.log("Enclave contract:", address(enclave));

        // Deploy CiphernodeRegistryOwnable contract
        console.log("Deploying CiphernodeRegistryOwnable contract...");
        CiphernodeRegistryOwnable cypherNodeRegistry = new CiphernodeRegistryOwnable(
                deployer,
                address(enclave)
            );
        console.log(
            "CiphernodeRegistryOwnable contract:",
            address(cypherNodeRegistry)
        );

        // Deploy NaiveRegistryFilter contract
        console.log("Deploying NaiveRegistryFilter contract...");
        NaiveRegistryFilter naiveRegistryFilter = new NaiveRegistryFilter(
            deployer,
            address(cypherNodeRegistry)
        );
        console.log(
            "NaiveRegistryFilter contract:",
            address(naiveRegistryFilter)
        );

        // Set registry in enclave
        address registryAddress = enclave.ciphernodeRegistry();
        if (registryAddress != address(cypherNodeRegistry)) {
            console.log("Setting registry in Enclave contract...");
            enclave.setCiphernodeRegistry(address(cypherNodeRegistry));
            console.log("Enclave contract updated with registry");
        } else {
            console.log("Enclave contract already has registry");
        }

        vm.stopBroadcast();
    }
}
