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
  submissionWindow?: number;
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
  submissionWindow,
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
    !submissionWindow ||
    (preDeployedArgs?.constructorArgs?.enclaveAddress === enclaveAddress &&
      preDeployedArgs?.constructorArgs?.owner === owner &&
      preDeployedArgs?.constructorArgs?.submissionWindow ===
        submissionWindow.toString())
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

  const ciphernodeRegistry = await ciphernodeRegistryFactory.deploy();
  await ciphernodeRegistry.waitForDeployment();
  const blockNumber = await ethers.provider.getBlockNumber();
  const ciphernodeRegistryAddress = await ciphernodeRegistry.getAddress();

  const initData = ciphernodeRegistryFactory.interface.encodeFunctionData(
    "initialize",
    [owner, enclaveAddress, submissionWindow],
  );

  const ProxyCF = await ethers.getContractFactory(
    "TransparentUpgradeableProxy",
  );
  const proxy = await ProxyCF.deploy(
    ciphernodeRegistryAddress,
    signer,
    initData,
  );
  await proxy.waitForDeployment();
  const proxyAddress = await proxy.getAddress();

  storeDeploymentArgs(
    {
      constructorArgs: {
        owner,
        enclaveAddress: enclaveAddress,
        submissionWindow: submissionWindow.toString(),
      },
      blockNumber,
      address: proxyAddress,
      implementationAddress: ciphernodeRegistryAddress,
    },
    "CiphernodeRegistryOwnable",
    chain,
  );

  const ciphernodeRegistryContract = CiphernodeRegistryOwnableFactory.connect(
    proxyAddress,
    signer,
  );

  return { ciphernodeRegistry: ciphernodeRegistryContract };
};

export const upgradeAndSaveCiphernodeRegistryOwnable = async ({
  poseidonT3Address,
  proxyAdminAddress,
  hre,
}: {
  poseidonT3Address: string;
  proxyAdminAddress: string;
  hre: HardhatRuntimeEnvironment;
}): Promise<{
  ciphernodeRegistry: CiphernodeRegistryOwnable;
  implementationAddress: string;
}> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = hre.globalOptions.network;

  const preDeployedArgs = readDeploymentArgs(
    "CiphernodeRegistryOwnable",
    chain,
  );
  if (!preDeployedArgs?.address) {
    throw new Error(
      "CiphernodeRegistryOwnable proxy not found. Deploy first before upgrading.",
    );
  }

  const proxyAddress = preDeployedArgs.address;

  const ciphernodeRegistryFactory = await ethers.getContractFactory(
    CiphernodeRegistryOwnableFactory.abi,
    CiphernodeRegistryOwnableFactory.linkBytecode({
      "npm/poseidon-solidity@0.0.5/PoseidonT3.sol:PoseidonT3":
        poseidonT3Address,
    }),
    signer,
  );

  const newImplementation = await ciphernodeRegistryFactory.deploy();
  await newImplementation.waitForDeployment();
  const newImplementationAddress = await newImplementation.getAddress();

  const proxyAdmin = await ethers.getContractAt(
    "ProxyAdmin",
    proxyAdminAddress,
    signer,
  );
  const upgradeTx = await proxyAdmin.upgrade(
    proxyAddress,
    newImplementationAddress,
  );
  await upgradeTx.wait();

  storeDeploymentArgs(
    {
      ...preDeployedArgs,
      implementationAddress: newImplementationAddress,
    },
    "CiphernodeRegistryOwnable",
    chain,
  );

  const ciphernodeRegistryContract = CiphernodeRegistryOwnableFactory.connect(
    proxyAddress,
    signer,
  );
  return {
    ciphernodeRegistry: ciphernodeRegistryContract,
    implementationAddress: newImplementationAddress,
  };
};
