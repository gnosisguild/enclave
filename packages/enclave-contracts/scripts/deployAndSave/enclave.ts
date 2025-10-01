// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import EnclaveModule from "../../ignition/modules/enclave";
import { Enclave, Enclave__factory as EnclaveFactory } from "../../types";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

/**
 * The arguments for the deployAndSaveEnclave function
 */
export interface EnclaveArgs {
  params?: string;
  owner?: string;
  maxDuration?: string;
  registry?: string;
  bondingRegistry?: string;
  feeToken?: string;
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
  bondingRegistry,
  feeToken,
  hre,
}: EnclaveArgs): Promise<{ enclave: Enclave }> => {
  const { ignition, ethers } = await hre.network.connect();

  const [signer] = await ethers.getSigners();

  const chain = hre.globalOptions.network;
  const preDeployedArgs = readDeploymentArgs("Enclave", chain);

  if (
    !params ||
    !owner ||
    !maxDuration ||
    !registry ||
    !bondingRegistry ||
    !feeToken ||
    (preDeployedArgs?.constructorArgs?.params === params &&
      preDeployedArgs?.constructorArgs?.owner === owner &&
      preDeployedArgs?.constructorArgs?.maxDuration === maxDuration &&
      preDeployedArgs?.constructorArgs?.registry === registry &&
      preDeployedArgs?.constructorArgs?.bondingRegistry === bondingRegistry &&
      preDeployedArgs?.constructorArgs?.feeToken === feeToken)
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

  const enclave = await ignition.deploy(EnclaveModule, {
    parameters: {
      Enclave: {
        params,
        owner,
        maxDuration,
        registry,
        bondingRegistry,
        feeToken,
      },
    },
  });

  await enclave.enclave.waitForDeployment();

  const enclaveAddress = await enclave.enclave.getAddress();
  const blockNumber = await ethers.provider.getBlockNumber();

  storeDeploymentArgs(
    {
      constructorArgs: {
        params,
        owner,
        maxDuration,
        registry,
        bondingRegistry,
        feeToken,
      },
      blockNumber,
      address: enclaveAddress,
    },
    "Enclave",
    chain,
  );

  const enclaveContract = EnclaveFactory.connect(enclaveAddress, signer);

  return { enclave: enclaveContract };
};
