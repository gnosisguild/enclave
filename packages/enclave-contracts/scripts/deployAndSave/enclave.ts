// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import { Enclave, Enclave__factory as EnclaveFactory } from "../../types";
import {
  areArraysEqual,
  readDeploymentArgs,
  storeDeploymentArgs,
} from "../utils";

/**
 * The arguments for the deployAndSaveEnclave function
 */
export interface EnclaveArgs {
  params?: string[];
  owner?: string;
  maxDuration?: string;
  registry?: string;
  poseidonT3Address: string;
  hre: HardhatRuntimeEnvironment;
}

/**
 * Deploys the Enclave contract and saves the deployment arguments
 * @param param0 - The deployment arguments
 * @returns The deployed Enclave contract
 */
export const deployAndSaveEnclave = async ({
  params,
  owner,
  maxDuration,
  registry,
  poseidonT3Address,
  hre,
}: EnclaveArgs): Promise<{ enclave: Enclave }> => {
  const { ethers } = await hre.network.connect();

  const [signer] = await ethers.getSigners();

  const chain = hre.globalOptions.network;
  const preDeployedArgs = readDeploymentArgs("Enclave", chain);

  if (
    !params ||
    !owner ||
    !maxDuration ||
    !registry ||
    (preDeployedArgs?.constructorArgs?.owner === owner &&
      preDeployedArgs?.constructorArgs?.maxDuration === maxDuration &&
      preDeployedArgs?.constructorArgs?.registry === registry &&
      areArraysEqual(
        preDeployedArgs?.constructorArgs?.params as string[],
        params,
      ))
  ) {
    if (!preDeployedArgs?.address) {
      throw new Error("Enclave address not found, it must be deployed first");
    }
    const enclaveContract = EnclaveFactory.connect(
      preDeployedArgs.address,
      signer,
    );
    return { enclave: enclaveContract };
  }

  const enclaveFactory = await ethers.getContractFactory(
    EnclaveFactory.abi,
    EnclaveFactory.linkBytecode({
      "npm/poseidon-solidity@0.0.5/PoseidonT3.sol:PoseidonT3":
        poseidonT3Address,
    }),
    signer,
  );

  const enclave = await enclaveFactory.deploy(
    owner,
    registry,
    maxDuration,
    params,
  );

  await enclave.waitForDeployment();

  const enclaveAddress = await enclave.getAddress();
  const blockNumber = await ethers.provider.getBlockNumber();

  storeDeploymentArgs(
    {
      constructorArgs: { owner, registry, maxDuration, params },
      blockNumber,
      address: enclaveAddress,
    },
    "Enclave",
    chain,
  );

  const enclaveContract = EnclaveFactory.connect(enclaveAddress, signer);

  return { enclave: enclaveContract };
};
