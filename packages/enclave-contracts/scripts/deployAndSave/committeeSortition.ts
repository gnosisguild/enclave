// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import {
  CommitteeSortition,
  CommitteeSortition__factory as CommitteeSortitionFactory,
} from "../../types";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

/**
 * The arguments for the deployAndSaveCommitteeSortition function
 */
export interface CommitteeSortitionArgs {
  bondingRegistry?: string;
  ciphernodeRegistry?: string;
  submissionWindow?: number;
  hre: HardhatRuntimeEnvironment;
}

/**
 * Deploys the CommitteeSortition contract and saves the deployment arguments
 * @param param0 - The deployment arguments
 * @returns The deployed CommitteeSortition contract
 */
export const deployAndSaveCommitteeSortition = async ({
  bondingRegistry,
  ciphernodeRegistry,
  submissionWindow = 300, // Default 5 minutes
  hre,
}: CommitteeSortitionArgs): Promise<{
  committeeSortition: CommitteeSortition;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const preDeployedArgs = readDeploymentArgs("CommitteeSortition", chain);

  if (
    !bondingRegistry ||
    !ciphernodeRegistry ||
    (preDeployedArgs?.constructorArgs?.bondingRegistry === bondingRegistry &&
      preDeployedArgs?.constructorArgs?.ciphernodeRegistry ===
        ciphernodeRegistry &&
      Number(preDeployedArgs?.constructorArgs?.submissionWindow) ===
        submissionWindow)
  ) {
    if (!preDeployedArgs?.address) {
      throw new Error(
        "CommitteeSortition address not found, it must be deployed first",
      );
    }
    const committeeSortitionContract = CommitteeSortitionFactory.connect(
      preDeployedArgs.address,
      signer,
    );
    return { committeeSortition: committeeSortitionContract };
  }

  const committeeSortitionFactory =
    await ethers.getContractFactory("CommitteeSortition");

  const committeeSortition = await committeeSortitionFactory.deploy(
    bondingRegistry,
    ciphernodeRegistry,
    submissionWindow,
  );

  await committeeSortition.waitForDeployment();

  const blockNumber = await ethers.provider.getBlockNumber();

  const committeeSortitionAddress = await committeeSortition.getAddress();

  storeDeploymentArgs(
    {
      constructorArgs: {
        bondingRegistry,
        ciphernodeRegistry,
        submissionWindow: submissionWindow.toString(),
      },
      blockNumber,
      address: committeeSortitionAddress,
    },
    "CommitteeSortition",
    chain,
  );

  const committeeSortitionContract = CommitteeSortitionFactory.connect(
    committeeSortitionAddress,
    signer,
  );

  return { committeeSortition: committeeSortitionContract };
};
