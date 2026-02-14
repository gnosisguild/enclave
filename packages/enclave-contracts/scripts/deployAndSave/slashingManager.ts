// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import {
  SlashingManager,
  SlashingManager__factory as SlashingManagerFactory,
} from "../../types";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

/**
 * The arguments for the deployAndSaveSlashingManager function
 */
export interface SlashingManagerArgs {
  admin?: string;
  bondingRegistry?: string;
  ciphernodeRegistry?: string;
  enclave?: string;
  hre: HardhatRuntimeEnvironment;
}

/**
 * Deploys the SlashingManager contract and saves the deployment arguments
 * @param param0 - The deployment arguments
 * @returns The deployed SlashingManager contract
 */
export const deployAndSaveSlashingManager = async ({
  admin,
  bondingRegistry,
  ciphernodeRegistry,
  enclave,
  hre,
}: SlashingManagerArgs): Promise<{
  slashingManager: SlashingManager;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const preDeployedArgs = readDeploymentArgs("SlashingManager", chain);

  if (
    !admin ||
    !bondingRegistry ||
    !ciphernodeRegistry ||
    !enclave ||
    (preDeployedArgs?.constructorArgs?.admin === admin &&
      preDeployedArgs?.constructorArgs?.bondingRegistry === bondingRegistry &&
      preDeployedArgs?.constructorArgs?.ciphernodeRegistry ===
        ciphernodeRegistry &&
      preDeployedArgs?.constructorArgs?.enclave === enclave)
  ) {
    if (!preDeployedArgs?.address) {
      throw new Error(
        "SlashingManager address not found, it must be deployed first",
      );
    }
    const slashingManagerContract = SlashingManagerFactory.connect(
      preDeployedArgs.address,
      signer,
    );
    return { slashingManager: slashingManagerContract };
  }

  const slashingManagerFactory =
    await ethers.getContractFactory("SlashingManager");
  const slashingManager = await slashingManagerFactory.deploy(
    admin,
    bondingRegistry,
    ciphernodeRegistry,
    enclave,
  );

  await slashingManager.waitForDeployment();

  const blockNumber = await ethers.provider.getBlockNumber();

  const slashingManagerAddress = await slashingManager.getAddress();

  storeDeploymentArgs(
    {
      constructorArgs: {
        admin,
        bondingRegistry,
        ciphernodeRegistry,
        enclave,
      },
      blockNumber,
      address: slashingManagerAddress,
    },
    "SlashingManager",
    chain,
  );

  const slashingManagerContract = SlashingManagerFactory.connect(
    slashingManagerAddress,
    signer,
  );

  return { slashingManager: slashingManagerContract };
};
