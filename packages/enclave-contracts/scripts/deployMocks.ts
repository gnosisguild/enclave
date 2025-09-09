// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import hre from "hardhat";

import { deployAndSaveMockComputeProvider } from "./deployAndSave/mockComputeProvider";
import { deployAndSaveMockDecryptionVerifier } from "./deployAndSave/mockDecryptionVerifier";
import { deployAndSaveMockInputValidator } from "./deployAndSave/mockInputValidator";
import { deployAndSaveMockProgram } from "./deployAndSave/mockProgram";

export interface MockDeployments {
  computeProviderAddress: string;
  decryptionVerifierAddress: string;
  inputValidatorAddress: string;
  e3ProgramAddress: string;
}

/**
 * Deploys the mock contracts and returns the addresses.
 * @param enclaveAddress - The address of the enclave contract.
 * @returns The addresses of the mock contracts.
 */
export const deployMocks = async (): Promise<MockDeployments> => {
  const { computeProvider } = await deployAndSaveMockComputeProvider(hre);

  const computeProviderAddress = await computeProvider.getAddress();

  const { decryptionVerifier } = await deployAndSaveMockDecryptionVerifier(hre);

  const decryptionVerifierAddress = await decryptionVerifier.getAddress();

  const { inputValidator } = await deployAndSaveMockInputValidator(hre);
  const inputValidatorAddress = await inputValidator.getAddress();

  const { e3Program } = await deployAndSaveMockProgram({
    mockInputValidator: inputValidatorAddress,
    hre,
  });

  const e3ProgramAddress = await e3Program.getAddress();

  console.log(`
        MockDeployments:
        ----------------------------------------------------------------------
        MockComputeProvider:${computeProviderAddress}
        MockDecryptionVerifier:${decryptionVerifierAddress}
        MockInputValidator:${inputValidatorAddress}
        MockE3Program:${e3ProgramAddress}
        `);

  return {
    computeProviderAddress,
    decryptionVerifierAddress,
    inputValidatorAddress,
    e3ProgramAddress,
  };
};
