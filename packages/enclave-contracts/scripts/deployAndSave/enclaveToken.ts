// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import {
  EnclaveToken,
  EnclaveToken__factory as EnclaveTokenFactory,
} from "../../types";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

/**
 * The arguments for the deployAndSaveEnclaveToken function
 */
export interface EnclaveTokenArgs {
  owner?: string;
  hre: HardhatRuntimeEnvironment;
}

/**
 * Disables transfer restrictions for local development
 */
async function disableTransferRestrictionsForLocal(
  contract: EnclaveToken,
  chain: string,
): Promise<void> {
  if (chain !== "localhost" && chain !== "hardhat") {
    return;
  }
  console.log("Disabling transfer restrictions for chain", chain);
  console.log("Contract address", await contract.getAddress());

  try {
    const isRestricted = await contract.transfersRestricted();
    if (isRestricted) {
      const tx = await contract.setTransferRestriction(false);
      await tx.wait();
      console.log("Transfer restrictions disabled for local development");
    }
  } catch (error) {
    console.warn("Failed to disable transfer restrictions:", error);
  }
}

/**
 * Deploys the EnclaveToken contract and saves the deployment arguments
 * @param param0 - The deployment arguments
 * @returns The deployed EnclaveToken contract
 */
export const deployAndSaveEnclaveToken = async ({
  owner,
  hre,
}: EnclaveTokenArgs): Promise<{
  enclaveToken: EnclaveToken;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const preDeployedArgs = readDeploymentArgs("EnclaveToken", chain);

  if (!owner || preDeployedArgs?.constructorArgs?.owner === owner) {
    if (!preDeployedArgs?.address) {
      throw new Error(
        "EnclaveToken address not found, it must be deployed first",
      );
    }
    const enclaveTokenContract = EnclaveTokenFactory.connect(
      preDeployedArgs.address,
      signer,
    );

    await disableTransferRestrictionsForLocal(enclaveTokenContract, chain);

    return { enclaveToken: enclaveTokenContract };
  }

  const enclaveTokenFactory = await ethers.getContractFactory("EnclaveToken");
  const enclaveToken = await enclaveTokenFactory.deploy(owner);

  await enclaveToken.waitForDeployment();

  const blockNumber = await ethers.provider.getBlockNumber();

  const enclaveTokenAddress = await enclaveToken.getAddress();

  storeDeploymentArgs(
    {
      constructorArgs: {
        owner,
      },
      blockNumber,
      address: enclaveTokenAddress,
    },
    "EnclaveToken",
    chain,
  );

  const enclaveTokenContract = EnclaveTokenFactory.connect(
    enclaveTokenAddress,
    signer,
  );

  await disableTransferRestrictionsForLocal(enclaveTokenContract, chain);

  return { enclaveToken: enclaveTokenContract };
};
