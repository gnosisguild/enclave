// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import MockInputValidatorModule from "../../ignition/modules/mockInputValidator";
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
  const { ignition, ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const inputValidator = await ignition.deploy(MockInputValidatorModule);
  await inputValidator.mockInputValidator.waitForDeployment();
  const inputValidatorAddress =
    await inputValidator.mockInputValidator.getAddress();

  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";
  const blockNumber = await signer.provider?.getBlockNumber();

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
