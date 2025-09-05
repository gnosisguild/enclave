import { network } from "hardhat";

import MockInputValidatorModule from "../../ignition/modules/mockInputValidator";
import {
  MockInputValidator,
  MockInputValidator__factory as MockInputValidatorFactory,
} from "../../types";
import { storeDeploymentArgs } from "../utils";

export const deployAndSaveMockInputValidator = async (): Promise<{
  inputValidator: MockInputValidator;
}> => {
  const { ignition, ethers } = await network.connect();
  const [signer] = await ethers.getSigners();
  const inputValidator = await ignition.deploy(MockInputValidatorModule);
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
