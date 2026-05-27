// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import {
  InterfoldVestingEscrow,
  InterfoldVestingEscrow__factory as InterfoldVestingEscrowFactory,
} from "../../types";
import {
  getDeploymentChain,
  readDeploymentArgs,
  storeDeploymentArgs,
} from "../utils";

/**
 * The arguments for the deployAndSaveInterfoldVestingEscrow function.
 */
export interface InterfoldVestingEscrowArgs {
  token?: string;
  bondingRegistry?: string;
  tgeTimestamp?: string;
  owner?: string;
  hre: HardhatRuntimeEnvironment;
}

/**
 * Deploys the InterfoldVestingEscrow contract and saves the deployment arguments.
 */
export const deployAndSaveInterfoldVestingEscrow = async ({
  token,
  bondingRegistry,
  tgeTimestamp,
  owner,
  hre,
}: InterfoldVestingEscrowArgs): Promise<{
  interfoldVestingEscrow: InterfoldVestingEscrow;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = getDeploymentChain(hre);

  const preDeployedArgs = readDeploymentArgs("InterfoldVestingEscrow", chain);

  if (
    !token ||
    !bondingRegistry ||
    !tgeTimestamp ||
    !owner ||
    (preDeployedArgs?.constructorArgs?.token === token &&
      preDeployedArgs?.constructorArgs?.bondingRegistry === bondingRegistry &&
      preDeployedArgs?.constructorArgs?.tgeTimestamp === tgeTimestamp &&
      preDeployedArgs?.constructorArgs?.owner === owner)
  ) {
    if (!preDeployedArgs?.address) {
      throw new Error(
        "InterfoldVestingEscrow address not found, it must be deployed first",
      );
    }
    const interfoldVestingEscrow = InterfoldVestingEscrowFactory.connect(
      preDeployedArgs.address,
      signer,
    );
    return { interfoldVestingEscrow };
  }

  const interfoldVestingEscrowFactory = await ethers.getContractFactory(
    "InterfoldVestingEscrow",
  );
  const interfoldVestingEscrow = await interfoldVestingEscrowFactory.deploy(
    token,
    bondingRegistry,
    tgeTimestamp,
    owner,
  );
  await interfoldVestingEscrow.waitForDeployment();

  const blockNumber = await ethers.provider.getBlockNumber();
  const interfoldVestingEscrowAddress =
    await interfoldVestingEscrow.getAddress();

  storeDeploymentArgs(
    {
      constructorArgs: {
        token,
        bondingRegistry,
        tgeTimestamp,
        owner,
      },
      blockNumber,
      address: interfoldVestingEscrowAddress,
    },
    "InterfoldVestingEscrow",
    chain,
  );

  const interfoldVestingEscrowContract = InterfoldVestingEscrowFactory.connect(
    interfoldVestingEscrowAddress,
    signer,
  );

  return { interfoldVestingEscrow: interfoldVestingEscrowContract };
};
