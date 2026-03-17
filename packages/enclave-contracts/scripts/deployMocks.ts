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
  decryptionVerifierAddress: string | undefined;
  e3ProgramAddress: string;
}

/**
 * Deploys the mock contracts and returns the addresses.
 * @param shouldHaveZKVerification - When true, skips MockDecryptionVerifier (real BfvDecryptionVerifier will be used).
 */
export const deployMocks = async (
  shouldHaveZKVerification?: boolean,
): Promise<MockDeployments> => {
  console.log("Deploying Compute Provider");
  const { computeProvider } = await deployAndSaveMockComputeProvider(hre);

  const computeProviderAddress = await computeProvider.getAddress();

  let decryptionVerifierAddress: string | undefined;
  if (!shouldHaveZKVerification) {
    console.log("Deploying Decryption Verifier");
    const { decryptionVerifier } =
      await deployAndSaveMockDecryptionVerifier(hre);
    decryptionVerifierAddress = await decryptionVerifier.getAddress();
  }

  console.log("Deploying E3 Program");
  const { e3Program } = await deployAndSaveMockProgram({
    hre,
  });

  const e3ProgramAddress = await e3Program.getAddress();

  console.log(`
        MockDeployments:
        ----------------------------------------------------------------------
        MockComputeProvider:${computeProviderAddress}
        MockDecryptionVerifier:${decryptionVerifierAddress ?? "(skipped - using real ZK)"}
        MockE3Program:${e3ProgramAddress}
        `);

  return {
    computeProviderAddress,
    decryptionVerifierAddress,
    e3ProgramAddress,
  };
};
