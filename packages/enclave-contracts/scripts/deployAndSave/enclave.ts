import { network } from "hardhat";

import EnclaveModule from "../../ignition/modules/enclave";
import { Enclave, Enclave__factory as EnclaveFactory } from "../../types";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

/**
 * The arguments for the deployAndSaveEnclave function
 */
export interface EnclaveArgs {
  params: string;
  owner: string;
  maxDuration: string;
  registry: string;
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
}: EnclaveArgs): Promise<{ enclave: Enclave }> => {
  const { ignition, ethers } = await network.connect();

  const [signer] = await ethers.getSigners();

  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const preDeployedArgs = readDeploymentArgs("Enclave", chain);

  if (
    preDeployedArgs?.constructorArgs?.params === params &&
    preDeployedArgs?.constructorArgs?.owner === owner &&
    preDeployedArgs?.constructorArgs?.maxDuration === maxDuration &&
    preDeployedArgs?.constructorArgs?.registry === registry
  ) {
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
      },
    },
  });

  const enclaveAddress = await enclave.enclave.getAddress();
  const blockNumber = await signer.provider?.getBlockNumber();

  storeDeploymentArgs(
    {
      constructorArgs: { params, owner, maxDuration, registry },
      blockNumber,
      address: enclaveAddress,
    },
    "Enclave",
    chain,
  );

  const enclaveContract = EnclaveFactory.connect(enclaveAddress, signer);

  return { enclave: enclaveContract };
};
