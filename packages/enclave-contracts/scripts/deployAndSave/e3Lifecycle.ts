// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import {
  E3Lifecycle,
  E3Lifecycle__factory as E3LifecycleFactory,
} from "../../types";
import { getProxyAdmin } from "../proxy";
import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

/**
 * E3 Timeout configuration
 */
export interface E3TimeoutConfig {
  committeeFormationWindow: number;
  dkgWindow: number;
  computeWindow: number;
  decryptionWindow: number;
  gracePeriod: number;
}

/**
 * The arguments for the deployAndSaveE3Lifecycle function
 */
export interface E3LifecycleArgs {
  owner?: string;
  enclave?: string;
  timeoutConfig?: E3TimeoutConfig;
  hre: HardhatRuntimeEnvironment;
}

/**
 * Default timeout configuration (in seconds)
 */
export const DEFAULT_TIMEOUT_CONFIG: E3TimeoutConfig = {
  committeeFormationWindow: 3600,
  dkgWindow: 7200,
  computeWindow: 86400,
  decryptionWindow: 3600,
  gracePeriod: 600,
};

/**
 * Deploys the E3Lifecycle contract and saves the deployment arguments
 * @param param0 - The deployment arguments
 * @returns The deployed E3Lifecycle contract
 */
export const deployAndSaveE3Lifecycle = async ({
  owner,
  enclave,
  timeoutConfig = DEFAULT_TIMEOUT_CONFIG,
  hre,
}: E3LifecycleArgs): Promise<{ e3Lifecycle: E3Lifecycle }> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = hre.globalOptions.network;

  const preDeployedArgs = readDeploymentArgs("E3Lifecycle", chain);

  if (
    !owner ||
    !enclave ||
    (preDeployedArgs?.constructorArgs?.owner === owner &&
      preDeployedArgs?.constructorArgs?.enclave === enclave)
  ) {
    if (!preDeployedArgs?.address) {
      throw new Error(
        "E3Lifecycle address not found, it must be deployed first",
      );
    }
    const e3LifecycleContract = E3LifecycleFactory.connect(
      preDeployedArgs.address,
      signer,
    );
    return { e3Lifecycle: e3LifecycleContract };
  }

  const e3LifecycleFactory = await ethers.getContractFactory(
    E3LifecycleFactory.abi,
    E3LifecycleFactory.bytecode,
    signer,
  );
  const e3Lifecycle = await e3LifecycleFactory.deploy();
  await e3Lifecycle.waitForDeployment();

  const blockNumber = await ethers.provider.getBlockNumber();
  const e3LifecycleAddress = await e3Lifecycle.getAddress();

  const initData = e3LifecycleFactory.interface.encodeFunctionData(
    "initialize",
    [owner, enclave, timeoutConfig],
  );

  const ProxyCF = await ethers.getContractFactory(
    "TransparentUpgradeableProxy",
  );
  const proxy = await ProxyCF.deploy(e3LifecycleAddress, owner, initData);
  await proxy.waitForDeployment();
  const proxyAddress = await proxy.getAddress();

  const proxyAdminAddress = await getProxyAdmin(ethers.provider, proxyAddress);

  storeDeploymentArgs(
    {
      constructorArgs: {
        owner,
        enclave,
        timeoutConfig: JSON.stringify(timeoutConfig),
      },
      proxyRecords: {
        initData,
        initialOwner: owner,
        proxyAddress,
        proxyAdminAddress,
        implementationAddress: e3LifecycleAddress,
      },
      blockNumber,
      address: proxyAddress,
    },
    "E3Lifecycle",
    chain,
  );

  const e3LifecycleContract = E3LifecycleFactory.connect(proxyAddress, signer);

  return { e3Lifecycle: e3LifecycleContract };
};
