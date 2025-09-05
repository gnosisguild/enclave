import { network } from "hardhat";

import MockComputeProviderModule from "../../ignition/modules/mockComputeProvider";
import {
  MockComputeProvider,
  MockComputeProvider__factory as MockComputeProviderFactory,
} from "../../types";
import { storeDeploymentArgs } from "../utils";

export const deployAndSaveMockComputeProvider = async (): Promise<{
  computeProvider: MockComputeProvider;
}> => {
  const { ignition, ethers } = await network.connect();

  const computeProvider = await ignition.deploy(MockComputeProviderModule);

  const computeProviderAddress =
    await computeProvider.mockComputeProvider.getAddress();

  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";
  const blockNumber = await signer.provider?.getBlockNumber();

  storeDeploymentArgs(
    {
      blockNumber,
      address: computeProviderAddress,
    },
    "MockComputeProvider",
    chain,
  );

  const computeProviderContract = MockComputeProviderFactory.connect(
    computeProviderAddress,
    signer,
  );

  return { computeProvider: computeProviderContract };
};
