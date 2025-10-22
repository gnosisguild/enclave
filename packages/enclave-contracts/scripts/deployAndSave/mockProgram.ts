// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import {
  MockE3Program,
  MockE3Program__factory as MockE3ProgramFactory,
} from "../../types";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

interface MockProgramArgs {
  mockInputValidator: string;
  hre: HardhatRuntimeEnvironment;
}

export const deployAndSaveMockProgram = async ({
  mockInputValidator,
  hre,
}: MockProgramArgs): Promise<{
  e3Program: MockE3Program;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const preDeployedArgs = readDeploymentArgs("MockE3Program", chain);

  if (
    preDeployedArgs?.constructorArgs?.mockInputValidator === mockInputValidator
  ) {
    const e3ProgramContract = MockE3ProgramFactory.connect(
      preDeployedArgs.address,
      signer,
    );
    return { e3Program: e3ProgramContract };
  }

  const e3ProgramFactory = await ethers.getContractFactory("MockE3Program");
  const e3Program = await e3ProgramFactory.deploy(mockInputValidator);

  await e3Program.waitForDeployment();

  const e3ProgramAddress = await e3Program.getAddress();
  const blockNumber = await ethers.provider.getBlockNumber();

  storeDeploymentArgs(
    {
      constructorArgs: { mockInputValidator },
      blockNumber,
      address: e3ProgramAddress,
    },
    "MockE3Program",
    chain,
  );

  const mockProgramContract = MockE3ProgramFactory.connect(
    e3ProgramAddress,
    signer,
  );

  return { e3Program: mockProgramContract };
};
