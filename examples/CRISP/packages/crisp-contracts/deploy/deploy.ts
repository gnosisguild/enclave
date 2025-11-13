// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { deployEnclave, updateE3Config } from "@enclave-e3/contracts/scripts";
import { deployCRISPContracts } from "./crisp";
import path from "path";

import hre from "hardhat";
import { fileURLToPath } from "url";

// Map contract names to config keys
const contractMapping: Record<string, string> = {
    CRISPProgram: "e3_program",
    Enclave: "enclave",
    CiphernodeRegistryOwnable: "ciphernode_registry",
    BondingRegistry: "bonding_registry",
    MockUSDC: "fee_token",
};

// Get __dirname equivalent in ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * Deploys the Enclave and CRISP contracts
 */
export const deploy = async () => {
    const chain = hre.globalOptions.network;

    const shouldDeployEnclave = Boolean(process.env.DEPLOY_ENCLAVE) ?? false;

    if (shouldDeployEnclave) {
        await deployEnclave(true);
    }
    await deployCRISPContracts();

    // this expects you to run it from CRISP's root
    console.log("path:", path.join(__dirname, "..", "..", "..", "enclave.config.yaml"));
    updateE3Config(chain, path.join(__dirname, "..", "..", "..", "enclave.config.yaml"), contractMapping);
}

deploy().catch((err => {
    console.log(err);
}))