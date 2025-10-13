// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { network } from "hardhat";
import { zeroAddress, zeroHash } from "viem";

import assert from "node:assert/strict";
import { describe, it, before } from "node:test";


describe("CRISP Contracts", async function () {
    const { viem: viemHardhat } = await network.connect();
    const publicClient = await viemHardhat.getPublicClient();

    const nonZeroAddress = "0xc6e7DF5E7b4f2A278906862b61205850344D4e7d";

    describe("deployment", () => {
        it("should deploy the contracts", async () => {
            /*
                IEnclave _enclave,
                IRiscZeroVerifier _verifier,
                ISemaphore _semaphore,
                CRISPCheckerFactory _checkerFactory,
                CRISPPolicyFactory _policyFactory,
                CRISPInputValidatorFactory _inputValidatorFactory,
                HonkVerifier _honkVerifier,
                bytes32 _imageId
            */
            const program = await viemHardhat.deployContract("CRISPProgram", [
                nonZeroAddress,
                nonZeroAddress,
                nonZeroAddress,
                nonZeroAddress,
                nonZeroAddress,
                nonZeroAddress,
                nonZeroAddress,
                zeroHash
            ])
            
            assert(program.address !== zeroAddress)
        })
    })
})
