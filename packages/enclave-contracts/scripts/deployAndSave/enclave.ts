// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import { Enclave, Enclave__factory as EnclaveFactory } from "../../types";
import { getProxyAdmin, verifyProxyAdminOwner } from "../proxy";
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
  const { ethers } = await hre.network.connect();

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
    (preDeployedArgs?.constructorArgs?.owner === owner &&
      preDeployedArgs?.constructorArgs?.maxDuration === maxDuration &&
      preDeployedArgs?.constructorArgs?.registry === registry &&
      preDeployedArgs?.constructorArgs?.bondingRegistry === bondingRegistry &&
      preDeployedArgs?.constructorArgs?.feeToken === feeToken &&
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

  const enclaveFactory = await ethers.getContractFactory("Enclave", signer);

  const enclave = await enclaveFactory.deploy();
  await enclave.waitForDeployment();
  const blockNumber = await ethers.provider.getBlockNumber();
  const enclaveAddress = await enclave.getAddress();

  const initData = enclaveFactory.interface.encodeFunctionData("initialize", [
    owner,
    registry,
    bondingRegistry,
    feeToken,
    maxDuration,
    params,
  ]);

  const ProxyCF = await ethers.getContractFactory(
    "TransparentUpgradeableProxy",
  );
  const proxy = await ProxyCF.deploy(enclaveAddress, owner, initData);
  await proxy.waitForDeployment();
  const proxyAddress = await proxy.getAddress();

  const proxyAdminAddress = await getProxyAdmin(ethers.provider, proxyAddress);

  storeDeploymentArgs(
    {
      constructorArgs: {
        owner,
        registry,
        bondingRegistry,
        feeToken,
        maxDuration,
        params,
      },
      proxyRecords: {
        initData,
        initialOwner: owner,
        proxyAddress,
        proxyAdminAddress,
        implementationAddress: enclaveAddress,
      },
      blockNumber,
      address: proxyAddress,
    },
    "Enclave",
    chain,
  );

  const enclaveContract = EnclaveFactory.connect(proxyAddress, signer);

  return { enclave: enclaveContract };
};

/**
 * Upgrades the Enclave implementation while keeping the same proxy address
 * @param param0 - The upgrade arguments
 * @returns The upgraded Enclave contract (same proxy address)
 */
export const upgradeAndSaveEnclave = async ({
  poseidonT3Address,
  ownerAddress,
  hre,
}: {
  poseidonT3Address: string;
  ownerAddress: string;
  hre: HardhatRuntimeEnvironment;
}): Promise<{ enclave: Enclave; implementationAddress: string }> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = hre.globalOptions.network;

  const preDeployedArgs = readDeploymentArgs("Enclave", chain);
  if (!preDeployedArgs?.address) {
    throw new Error("Enclave proxy not found. Deploy first before upgrading.");
  }

  const proxyAddress = preDeployedArgs.address;

  const autoProxyAdminAddress = await getProxyAdmin(
    ethers.provider,
    proxyAddress,
  );
  console.log("Auto-deployed ProxyAdmin address:", autoProxyAdminAddress);

  const enclaveFactory = await ethers.getContractFactory("Enclave", signer);

  const newImplementation = await enclaveFactory.deploy();
  await newImplementation.waitForDeployment();
  const newImplementationAddress = await newImplementation.getAddress();
  console.log("New Implementation Address:", newImplementationAddress);

  const proxyAdmin = await ethers.getContractAt(
    "ProxyAdmin",
    autoProxyAdminAddress,
    signer,
  );
  await verifyProxyAdminOwner(proxyAdmin, ownerAddress);

  // TODO: Add init data if needed
  const initData = "0x";
  const upgradeTx = await proxyAdmin.upgradeAndCall(
    proxyAddress,
    newImplementationAddress,
    initData,
  );
  await upgradeTx.wait();

  const existingProxyRecords = preDeployedArgs.proxyRecords
    ? Object.fromEntries(
        Object.entries(preDeployedArgs.proxyRecords).filter(
          ([, value]) => value !== undefined,
        ),
      )
    : {};

  const proxyRecords: Record<string, string | string[]> = {
    ...existingProxyRecords,
    implementationAddress: newImplementationAddress,
  };

  if (initData !== "0x") {
    proxyRecords.initData = initData;
  }

  storeDeploymentArgs({ ...preDeployedArgs, proxyRecords }, "Enclave", chain);

  const enclaveContract = EnclaveFactory.connect(proxyAddress, signer);
  return {
    enclave: enclaveContract,
    implementationAddress: newImplementationAddress,
  };
};
