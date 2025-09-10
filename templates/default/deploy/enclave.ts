// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import deployEnclave from "@enclave-e3/contracts/deploy/enclave";
import deployMocks from "@enclave-e3/contracts/deploy/mocks";
import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

const func: DeployFunction = async function (hre: HardhatRuntimeEnvironment) {
    await deployEnclave(hre);
    // INFO: We need to deploy the mock contract due to the decryptionVerifier.
    // Once we have a real verifier, we can remove this.
    await deployMocks(hre);
};

export default func;
func.tags = ["enclave", "mocks"];
