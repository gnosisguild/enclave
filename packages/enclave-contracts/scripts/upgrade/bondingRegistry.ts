// SPDX-License-Identifier: LGPL-3.0-only
import hre from "hardhat";

import { upgradeAndSaveBondingRegistry } from "../deployAndSave/bondingRegistry";
import { readDeploymentArgs } from "../utils";

/**
 * Upgrades the Enclave contract implementation and saves the deployment arguments
 * This keeps the same proxy address, only updates the implementation
 */
export const upgradeBondingRegistry = async () => {
  const { ethers } = await hre.network.connect();
  const [owner] = await ethers.getSigners();
  const ownerAddress = await owner.getAddress();
  const chain = hre.globalOptions.network;
  console.log("Owner:", ownerAddress);

  const preDeployedArgs = readDeploymentArgs("BondingRegistry", chain);
  if (!preDeployedArgs?.address) {
    throw new Error(
      "BondingRegistry proxy not found. Deploy first before upgrading.",
    );
  }

  console.log(
    "BondingRegistry Proxy Address (from deployments):",
    preDeployedArgs.address,
  );

  const code = await ethers.provider.getCode(preDeployedArgs.address);
  if (code === "0x") {
    throw new Error(
      `No contract found at proxy address ${preDeployedArgs.address}`,
    );
  }
  console.log("Proxy contract exists on-chain");

  const { bondingRegistry, implementationAddress } =
    await upgradeAndSaveBondingRegistry({
      proxyAdminAddress: ownerAddress,
      hre,
    });

  const bondingRegistryAddress = await bondingRegistry.getAddress();

  console.log(`
    ============================================
    Upgrade Complete!
    ============================================
    Proxy Address: ${bondingRegistryAddress}
    New Implementation: ${implementationAddress}
    ============================================
  `);
};

upgradeBondingRegistry().catch((error) => {
  console.error(error);
  process.exit(1);
});
