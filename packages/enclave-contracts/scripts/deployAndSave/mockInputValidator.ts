// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import {
  MockInputValidator,
  MockInputValidator__factory as MockInputValidatorFactory,
} from "../../types";
import { storeDeploymentArgs } from "../utils";

export const deployAndSaveMockInputValidator = async (
  hre: HardhatRuntimeEnvironment,
): Promise<{
  inputValidator: MockInputValidator;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();

  const mockInputValidatorFactory = await ethers.getContractFactory(
    "MockInputValidator",
  );
  const inputValidator = await mockInputValidatorFactory.deploy();
  await inputValidator.waitForDeployment();
  const inputValidatorAddress =
    await inputValidator.getAddress();

  const chain = hre.globalOptions.network;
  const blockNumber = await ethers.provider.getBlockNumber();

  storeDeploymentArgs(
    {
      blockNumber,
      address: inputValidatorAddress,
    },
    "MockInputValidator",
    chain,
  );

  const inputValidatorContract = MockInputValidatorFactory.connect(
    inputValidatorAddress,
    signer,
  );

  return { inputValidator: inputValidatorContract };
};
