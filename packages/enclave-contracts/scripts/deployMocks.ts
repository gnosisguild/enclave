// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

import MockComputeProviderModule from "../ignition/modules/mockComputeProvider";
import MockDecryptionVerifierModule from "../ignition/modules/mockDecryptionVerifier";
import MockE3ProgramModule from "../ignition/modules/mockE3Program";
import MockInputValidatorModule from "../ignition/modules/mockInputValidator";

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
  const { ignition } = await network.connect();

  const computeProvider = await ignition.deploy(MockComputeProviderModule);
  const computeProviderAddress =
    await computeProvider.mockComputeProvider.getAddress();

  const decryptionVerifier = await ignition.deploy(
    MockDecryptionVerifierModule,
  );
  const decryptionVerifierAddress =
    await decryptionVerifier.mockDecryptionVerifier.getAddress();

  const inputValidator = await ignition.deploy(MockInputValidatorModule);
  const inputValidatorAddress =
    await inputValidator.mockInputValidator.getAddress();

  const e3Program = await ignition.deploy(MockE3ProgramModule, {
    parameters: {
      MockE3Program: {
        mockInputValidator: inputValidatorAddress,
      },
    },
  });
  const e3ProgramAddress = await e3Program.mockE3Program.getAddress();

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
