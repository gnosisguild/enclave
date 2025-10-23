// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { network } from "hardhat";
import { zeroAddress, zeroHash } from "viem";

import assert from "node:assert/strict";
import { describe, it } from "node:test";


describe("CRISP Contracts", async function () {
    const { ethers } = await network.connect();

    const nonZeroAddress = "0xc6e7DF5E7b4f2A278906862b61205850344D4e7d";

    describe("deployment", () => {
        it("should deploy the contracts", async () => {
            /*
                IEnclave _enclave,
                IRiscZeroVerifier _verifier,
                CRISPInputValidatorFactory _inputValidatorFactory,
                HonkVerifier _honkVerifier,
                bytes32 _imageId
            */
            const program = await ethers.deployContract("CRISPProgram", [
                nonZeroAddress,
                nonZeroAddress,
                nonZeroAddress,
                nonZeroAddress,
                zeroHash
            ])
            
            assert(await program.getAddress() !== zeroAddress)
        })
    })
})
