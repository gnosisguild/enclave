// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

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
  poseidonT3Address: string;
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
  poseidonT3Address,
  hre,
}: CiphernodeRegistryOwnableArgs): Promise<{
  ciphernodeRegistry: CiphernodeRegistryOwnable;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const preDeployedArgs = readDeploymentArgs(
    "CiphernodeRegistryOwnable",
    chain,
  );

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

  const ciphernodeRegistryFactory = await ethers.getContractFactory(
    CiphernodeRegistryOwnableFactory.abi,
    CiphernodeRegistryOwnableFactory.linkBytecode({
      "npm/poseidon-solidity@0.0.5/PoseidonT3.sol:PoseidonT3":
        poseidonT3Address,
    }),
    signer,
  );

  const ciphernodeRegistry = await ciphernodeRegistryFactory.deploy(
    owner,
    enclaveAddress,
  );

  await ciphernodeRegistry.waitForDeployment();

  const blockNumber = await ethers.provider.getBlockNumber();

  const ciphernodeRegistryAddress = await ciphernodeRegistry.getAddress();

  storeDeploymentArgs(
    {
      constructorArgs: { owner, enclaveAddress: enclaveAddress },
      blockNumber,
      address: ciphernodeRegistryAddress,
    },
    "CiphernodeRegistryOwnable",
    chain,
  );

  const ciphernodeRegistryContract = CiphernodeRegistryOwnableFactory.connect(
    ciphernodeRegistryAddress,
    signer,
  );

  return { ciphernodeRegistry: ciphernodeRegistryContract };
};
