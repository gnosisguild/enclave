// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import hre from "hardhat";

import { deployAndSaveMockComputeProvider } from "./deployAndSave/mockComputeProvider";
import { deployAndSaveMockDecryptionVerifier } from "./deployAndSave/mockDecryptionVerifier";
import { deployAndSaveMockProgram } from "./deployAndSave/mockProgram";

export interface MockDeployments {
  computeProviderAddress: string;
  decryptionVerifierAddress: string;
  e3ProgramAddress: string;
}

/**
 * Deploys the mock contracts and returns the addresses.
 * @param enclaveAddress - The address of the enclave contract.
 * @returns The addresses of the mock contracts.
 */
export const deployMocks = async (): Promise<MockDeployments> => {
  console.log("Deploying Compute Provider");
  const { computeProvider } = await deployAndSaveMockComputeProvider(hre);

  const computeProviderAddress = await computeProvider.getAddress();

  console.log("Deploying Decryption Verifier");
  const { decryptionVerifier } = await deployAndSaveMockDecryptionVerifier(hre);

  const decryptionVerifierAddress = await decryptionVerifier.getAddress();

  console.log("Deploying E3 Program");
  const { e3Program } = await deployAndSaveMockProgram({
    hre,
  });

  const e3ProgramAddress = await e3Program.getAddress();

  console.log(`
        MockDeployments:
        ----------------------------------------------------------------------
        MockComputeProvider:${computeProviderAddress}
        MockDecryptionVerifier:${decryptionVerifierAddress}
        MockE3Program:${e3ProgramAddress}
        `);

  return {
    computeProviderAddress,
    decryptionVerifierAddress,
    e3ProgramAddress,
  };
};
