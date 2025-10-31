// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import {
  EnclaveTicketToken,
  EnclaveTicketToken__factory as EnclaveTicketTokenFactory,
} from "../../types";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

/**
 * The arguments for the deployAndSaveEnclaveTicketToken function
 */
export interface EnclaveTicketTokenArgs {
  baseToken?: string;
  registry?: string;
  owner?: string;
  hre: HardhatRuntimeEnvironment;
}

/**
 * Deploys the EnclaveTicketToken contract and saves the deployment arguments
 * @param param0 - The deployment arguments
 * @returns The deployed EnclaveTicketToken contract
 */
export const deployAndSaveEnclaveTicketToken = async ({
  baseToken,
  registry,
  owner,
  hre,
}: EnclaveTicketTokenArgs): Promise<{
  enclaveTicketToken: EnclaveTicketToken;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const preDeployedArgs = readDeploymentArgs("EnclaveTicketToken", chain);

  if (
    !baseToken ||
    !registry ||
    !owner ||
    (preDeployedArgs?.constructorArgs?.baseToken === baseToken &&
      preDeployedArgs?.constructorArgs?.registry === registry &&
      preDeployedArgs?.constructorArgs?.owner === owner)
  ) {
    if (!preDeployedArgs?.address) {
      throw new Error(
        "EnclaveTicketToken address not found, it must be deployed first",
      );
    }
    const enclaveTicketTokenContract = EnclaveTicketTokenFactory.connect(
      preDeployedArgs.address,
      signer,
    );
    return { enclaveTicketToken: enclaveTicketTokenContract };
  }

  const enclaveTicketTokenFactory =
    await ethers.getContractFactory("EnclaveTicketToken");
  const enclaveTicketToken = await enclaveTicketTokenFactory.deploy(
    baseToken,
    registry,
    owner,
  );

  await enclaveTicketToken.waitForDeployment();

  const blockNumber = await ethers.provider.getBlockNumber();

  const enclaveTicketTokenAddress = await enclaveTicketToken.getAddress();

  storeDeploymentArgs(
    {
      constructorArgs: {
        baseToken,
        registry,
        owner,
      },
      blockNumber,
      address: enclaveTicketTokenAddress,
    },
    "EnclaveTicketToken",
    chain,
  );

  const enclaveTicketTokenContract = EnclaveTicketTokenFactory.connect(
    enclaveTicketTokenAddress,
    signer,
  );

  return { enclaveTicketToken: enclaveTicketTokenContract };
};
