// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { deployEnclave } from "@enclave-e3/contracts/scripts";
import { deployCRISPContracts } from "./crisp";

/**
 * Deploys the Enclave and CRISP contracts
 */
export const deploy = async () => {
    const shouldDeployEnclave = Boolean(process.env.DEPLOY_ENCLAVE) ?? false;

    if (shouldDeployEnclave) {
        await deployEnclave(true);
    }
    await deployCRISPContracts();
}

deploy().catch((err => {
    console.log(err);
}))