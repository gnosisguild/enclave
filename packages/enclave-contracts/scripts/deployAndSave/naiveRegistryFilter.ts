// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import {
  NaiveRegistryFilter,
  NaiveRegistryFilter__factory as NaiveRegistryFilterFactory,
} from "../../types";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

export interface NaiveRegistryFilterArgs {
  ciphernodeRegistryAddress?: string;
  owner?: string;
  hre: HardhatRuntimeEnvironment;
}

export const deployAndSaveNaiveRegistryFilter = async ({
  ciphernodeRegistryAddress,
  owner,
  hre,
}: NaiveRegistryFilterArgs): Promise<{
  naiveRegistryFilter: NaiveRegistryFilter;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = hre.globalOptions.network;

  const preDeployedArgs = readDeploymentArgs("NaiveRegistryFilter", chain);
  if (
    !ciphernodeRegistryAddress ||
    !owner ||
    (preDeployedArgs?.constructorArgs?.ciphernodeRegistryAddress ===
      ciphernodeRegistryAddress &&
      preDeployedArgs?.constructorArgs?.owner === owner)
  ) {
    if (!preDeployedArgs?.address) {
      throw new Error(
        "NaiveRegistryFilter address not found, it must be deployed first",
      );
    }
    const naiveRegistryFilterContract = NaiveRegistryFilterFactory.connect(
      preDeployedArgs.address,
      signer,
    );
    return { naiveRegistryFilter: naiveRegistryFilterContract };
  }

  const naiveRegistryFilterFactory = await ethers.getContractFactory("NaiveRegistryFilter");

  const naiveRegistryFilter = await naiveRegistryFilterFactory.deploy(ciphernodeRegistryAddress, owner);

  await naiveRegistryFilter.waitForDeployment();

  const naiveRegistryFilterAddress =
    await naiveRegistryFilter.getAddress();

  const blockNumber = await ethers.provider.getBlockNumber();

  storeDeploymentArgs(
    {
      constructorArgs: {
        owner,
        ciphernodeRegistryAddress: ciphernodeRegistryAddress,
      },
      blockNumber,
      address: naiveRegistryFilterAddress,
    },
    "NaiveRegistryFilter",
    chain,
  );

  const naiveRegistryFilterContract = NaiveRegistryFilterFactory.connect(
    naiveRegistryFilterAddress,
    signer,
  );

  return { naiveRegistryFilter: naiveRegistryFilterContract };
};
