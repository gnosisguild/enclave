// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import BondingRegistryModule from "../../ignition/modules/bondingRegistry";
import {
  BondingRegistry,
  BondingRegistry__factory as BondingRegistryFactory,
} from "../../types";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

/**
 * The arguments for the deployAndSaveBondingRegistry function
 */
export interface BondingRegistryArgs {
  owner?: string;
  ticketToken?: string;
  licenseToken?: string;
  registry?: string;
  slashedFundsTreasury?: string;
  ticketPrice?: string;
  licenseRequiredBond?: string;
  minTicketBalance?: number;
  exitDelay?: number;
  hre: HardhatRuntimeEnvironment;
}

/**
 * Deploys the BondingRegistry contract and saves the deployment arguments
 * @param param0 - The deployment arguments
 * @returns The deployed BondingRegistry contract
 */
export const deployAndSaveBondingRegistry = async ({
  owner,
  ticketToken,
  licenseToken,
  registry,
  slashedFundsTreasury,
  ticketPrice,
  licenseRequiredBond,
  minTicketBalance,
  exitDelay,
  hre,
}: BondingRegistryArgs): Promise<{
  bondingRegistry: BondingRegistry;
}> => {
  const { ignition, ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const preDeployedArgs = readDeploymentArgs("BondingRegistry", chain);

  if (
    !owner ||
    !ticketToken ||
    !licenseToken ||
    !registry ||
    !slashedFundsTreasury ||
    !ticketPrice ||
    !licenseRequiredBond ||
    minTicketBalance === undefined ||
    exitDelay === undefined ||
    (preDeployedArgs?.constructorArgs?.owner === owner &&
      preDeployedArgs?.constructorArgs?.ticketToken === ticketToken &&
      preDeployedArgs?.constructorArgs?.licenseToken === licenseToken &&
      preDeployedArgs?.constructorArgs?.registry === registry &&
      preDeployedArgs?.constructorArgs?.slashedFundsTreasury ===
        slashedFundsTreasury &&
      preDeployedArgs?.constructorArgs?.ticketPrice === ticketPrice &&
      preDeployedArgs?.constructorArgs?.licenseRequiredBond ===
        licenseRequiredBond &&
      preDeployedArgs?.constructorArgs?.minTicketBalance ===
        minTicketBalance.toString() &&
      preDeployedArgs?.constructorArgs?.exitDelay === exitDelay.toString())
  ) {
    if (!preDeployedArgs?.address) {
      throw new Error(
        "BondingRegistry address not found, it must be deployed first",
      );
    }
    const bondingRegistryContract = BondingRegistryFactory.connect(
      preDeployedArgs.address,
      signer,
    );
    return { bondingRegistry: bondingRegistryContract };
  }

  const bondingRegistry = await ignition.deploy(BondingRegistryModule, {
    parameters: {
      BondingRegistry: {
        owner,
        ticketToken,
        licenseToken,
        registry,
        slashedFundsTreasury,
        ticketPrice,
        licenseRequiredBond,
        minTicketBalance,
        exitDelay,
      },
    },
  });

  await bondingRegistry.bondingRegistry.waitForDeployment();

  const blockNumber = await ethers.provider.getBlockNumber();

  const bondingRegistryAddress =
    await bondingRegistry.bondingRegistry.getAddress();

  storeDeploymentArgs(
    {
      constructorArgs: {
        owner,
        ticketToken,
        licenseToken,
        registry,
        slashedFundsTreasury,
        ticketPrice,
        licenseRequiredBond,
        minTicketBalance: minTicketBalance.toString(),
        exitDelay: exitDelay.toString(),
      },
      blockNumber,
      address: bondingRegistryAddress,
    },
    "BondingRegistry",
    chain,
  );

  const bondingRegistryContract = BondingRegistryFactory.connect(
    bondingRegistryAddress,
    signer,
  );

  return { bondingRegistry: bondingRegistryContract };
};
