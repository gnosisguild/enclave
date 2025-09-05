import { network } from "hardhat";

import NaiveRegistryFilterModule from "../../ignition/modules/naiveRegistryFilter";
import {
  NaiveRegistryFilter,
  NaiveRegistryFilter__factory as NaiveRegistryFilterFactory,
} from "../../types";
import { storeDeploymentArgs } from "../utils";

export interface NaiveRegistryFilterArgs {
  ciphernodeRegistryAddress: string;
  owner: string;
}

export const deployAndSaveNaiveRegistryFilter = async ({
  ciphernodeRegistryAddress,
  owner,
}: NaiveRegistryFilterArgs): Promise<{
  naiveRegistryFilter: NaiveRegistryFilter;
}> => {
  const { ignition, ethers } = await network.connect();
  const naiveRegistryFilter = await ignition.deploy(NaiveRegistryFilterModule, {
    parameters: {
      NaiveRegistryFilter: {
        ciphernodeRegistryAddress,
        owner,
      },
    },
  });

  const naiveRegistryFilterAddress =
    await naiveRegistryFilter.naiveRegistryFilter.getAddress();

  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name;
  const blockNumber = await signer.provider?.getBlockNumber();

  storeDeploymentArgs(
    {
      constructorArgs: {
        ciphernodeRegistryAddress: ciphernodeRegistryAddress,
        owner,
      },
      blockNumber,
      address: naiveRegistryFilterAddress,
    },
    "NaiveRegistryFilter",
    chain ?? "localhost",
  );

  const naiveRegistryFilterContract = NaiveRegistryFilterFactory.connect(
    naiveRegistryFilterAddress,
    signer,
  );

  return { naiveRegistryFilter: naiveRegistryFilterContract };
};
