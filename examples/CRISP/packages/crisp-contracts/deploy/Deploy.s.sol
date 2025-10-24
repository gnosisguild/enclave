// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

pragma solidity ^0.8.28;

import {RiscZeroGroth16Verifier} from "risc0/groth16/RiscZeroGroth16Verifier.sol";
import {ControlID} from "risc0/groth16/ControlID.sol";
import {Script} from "forge-std/Script.sol";
import "forge-std/Test.sol";

contract CRISPProgramDeploy is Script {
    function run() external {
        // Read and log the chainID
        uint256 chainId = block.chainid;
        console2.log("Deploying on ChainID %d", chainId);

        setupDeployer();
        setupVerifier();

        vm.stopBroadcast();
    }

    function setupDeployer() private {
        uint256 deployerKey = uint256(
            vm.envOr("PRIVATE_KEY", bytes32(0))
        );

        vm.startBroadcast(deployerKey);
    }

    function setupVerifier() private {
        RiscZeroGroth16Verifier verifier = new RiscZeroGroth16Verifier(
            ControlID.CONTROL_ROOT,
            ControlID.BN254_CONTROL_ID
        );
        console2.log(
            "Deployed RiscZeroGroth16Verifier to",
            address(verifier)
        );
    }
}