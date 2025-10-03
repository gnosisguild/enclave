// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import CiphernodeRegistryModule from "../../ignition/modules/ciphernodeRegistry";
import {
  CiphernodeRegistryOwnable,
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryOwnableFactory,
} from "../../types";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

/**
 * The arguments for the deployAndSaveCiphernodeRegistryOwnable function
 */
export interface CiphernodeRegistryOwnableArgs {
  enclaveAddress?: string;
  owner?: string;
  hre: HardhatRuntimeEnvironment;
}

/**
 * Deploys the CiphernodeRegistryOwnable contract and saves the deployment arguments
 * @param param0 - The deployment arguments
 * @returns The deployed CiphernodeRegistryOwnable contract
 */
export const deployAndSaveCiphernodeRegistryOwnable = async ({
  enclaveAddress,
  owner,
  hre,
}: CiphernodeRegistryOwnableArgs): Promise<{
  ciphernodeRegistry: CiphernodeRegistryOwnable;
}> => {
  const { ignition, ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const preDeployedArgs = readDeploymentArgs("CiphernodeRegistry", chain);

  if (
    !enclaveAddress ||
    !owner ||
    (preDeployedArgs?.constructorArgs?.enclaveAddress === enclaveAddress &&
      preDeployedArgs?.constructorArgs?.owner === owner)
  ) {
    if (!preDeployedArgs?.address) {
      throw new Error(
        "CiphernodeRegistry address not found, it must be deployed first",
      );
    }
    const ciphernodeRegistryContract = CiphernodeRegistryOwnableFactory.connect(
      preDeployedArgs.address,
      signer,
    );
    return { ciphernodeRegistry: ciphernodeRegistryContract };
  }

  const ciphernodeRegistry = await ignition.deploy(CiphernodeRegistryModule, {
    parameters: {
      CiphernodeRegistry: {
        enclaveAddress,
        owner,
      },
    },
  });

  await ciphernodeRegistry.cipherNodeRegistry.waitForDeployment();

  const blockNumber = await ethers.provider.getBlockNumber();

  const ciphernodeRegistryAddress =
    await ciphernodeRegistry.cipherNodeRegistry.getAddress();

  storeDeploymentArgs(
    {
      constructorArgs: { enclaveAddress: enclaveAddress, owner },
      blockNumber,
      address: ciphernodeRegistryAddress,
    },
    "CiphernodeRegistry",
    chain,
  );

  const ciphernodeRegistryContract = CiphernodeRegistryOwnableFactory.connect(
    ciphernodeRegistryAddress,
    signer,
  );

  return { ciphernodeRegistry: ciphernodeRegistryContract };
};
