// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { deployEnclave } from "@enclave-e3/contracts/deploy/enclave";
import { deployMocks } from "@enclave-e3/contracts/deploy/mocks";

export const deployContracts = async () => {
    // await deployEnclave();
    // // INFO: We need to deploy the mock contract due to the decryptionVerifier.
    // // Once we have a real verifier, we can remove this.
    // await deployMocks();
};

