// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import {
  InterfoldToken,
  InterfoldToken__factory as InterfoldTokenFactory,
} from "../../types";
import {
  getDeploymentChain,
  readDeploymentArgs,
  storeDeploymentArgs,
} from "../utils";

/**
 * The arguments for the deployAndSaveInterfoldToken function
 */
export interface InterfoldTokenArgs {
  owner?: string;
  hre: HardhatRuntimeEnvironment;
}

/**
 * Disables transfer restrictions for local development
 */
async function disableTransferRestrictionsForLocal(
  contract: InterfoldToken,
  chain: string,
): Promise<void> {
  if (chain !== "localhost" && chain !== "hardhat") {
    return;
  }
  console.log("Disabling transfer restrictions for chain", chain);
  console.log("Contract address", await contract.getAddress());

  try {
    const tgeTs = await contract.tgeTimestamp();
    if (tgeTs === 0n) {
      // Attempt TGE — will succeed only if CCA_END + 45 days has passed.
      // On local chains the deployer should advance time manually first.
      await (await contract.tge()).wait();
      console.log("TGE fired for local development");
    }
  } catch (error: any) {
    console.warn(
      "TGE not yet available (CCA cooldown may not have passed):",
      error.reason ?? error.message ?? error,
    );
  }
}

/**
 * Deploys the InterfoldToken contract and saves the deployment arguments
 * @param param0 - The deployment arguments
 * @returns The deployed InterfoldToken contract
 */
export const deployAndSaveInterfoldToken = async ({
  owner,
  hre,
}: InterfoldTokenArgs): Promise<{
  interfoldToken: InterfoldToken;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = getDeploymentChain(hre);

  const preDeployedArgs = readDeploymentArgs("InterfoldToken", chain);

  if (!owner || preDeployedArgs?.constructorArgs?.owner === owner) {
    if (!preDeployedArgs?.address) {
      throw new Error(
        "InterfoldToken address not found, it must be deployed first",
      );
    }
    const interfoldTokenContract = InterfoldTokenFactory.connect(
      preDeployedArgs.address,
      signer,
    );

    await disableTransferRestrictionsForLocal(interfoldTokenContract, chain);

    return { interfoldToken: interfoldTokenContract };
  }

  const interfoldTokenFactory =
    await ethers.getContractFactory("InterfoldToken");
  const interfoldToken = await interfoldTokenFactory.deploy(owner);

  await interfoldToken.waitForDeployment();

  const blockNumber = await ethers.provider.getBlockNumber();

  const interfoldTokenAddress = await interfoldToken.getAddress();

  storeDeploymentArgs(
    {
      constructorArgs: {
        owner,
      },
      blockNumber,
      address: interfoldTokenAddress,
    },
    "InterfoldToken",
    chain,
  );

  const interfoldTokenContract = InterfoldTokenFactory.connect(
    interfoldTokenAddress,
    signer,
  );

  await disableTransferRestrictionsForLocal(interfoldTokenContract, chain);

  return { interfoldToken: interfoldTokenContract };
};
