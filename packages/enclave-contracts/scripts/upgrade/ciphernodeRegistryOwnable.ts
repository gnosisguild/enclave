// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import hre from "hardhat";

import { upgradeAndSaveCiphernodeRegistryOwnable } from "../deployAndSave/ciphernodeRegistryOwnable";
import { deployAndSavePoseidonT3 } from "../deployAndSave/poseidonT3";
import { readDeploymentArgs } from "../utils";

/**
 * Upgrades the Enclave contract implementation and saves the deployment arguments
 * This keeps the same proxy address, only updates the implementation
 */
export const upgradeCiphernodeRegistryOwnable = async () => {
  const { ethers } = await hre.network.connect();
  const [owner] = await ethers.getSigners();
  const ownerAddress = await owner.getAddress();
  const chain = hre.globalOptions.network;
  console.log("Owner:", ownerAddress);

  const poseidonT3 = await deployAndSavePoseidonT3({ hre });
  const preDeployedArgs = readDeploymentArgs(
    "CiphernodeRegistryOwnable",
    chain,
  );
  if (!preDeployedArgs?.address) {
    throw new Error(
      "CiphernodeRegistryOwnable proxy not found. Deploy first before upgrading.",
    );
  }

  console.log(
    "CiphernodeRegistryOwnable Proxy Address (from deployments):",
    preDeployedArgs.address,
  );

  const code = await ethers.provider.getCode(preDeployedArgs.address);
  if (code === "0x") {
    throw new Error(
      `No contract found at proxy address ${preDeployedArgs.address}`,
    );
  }
  console.log("Proxy contract exists on-chain");

  const { ciphernodeRegistry, implementationAddress } =
    await upgradeAndSaveCiphernodeRegistryOwnable({
      poseidonT3Address: poseidonT3,
      ownerAddress: ownerAddress,
      hre,
    });

  const ciphernodeRegistryAddress = await ciphernodeRegistry.getAddress();

  console.log(`
    ============================================
    Upgrade Complete!
    ============================================
    Proxy Address: ${ciphernodeRegistryAddress}
    New Implementation: ${implementationAddress}
    ============================================
  `);
};

upgradeCiphernodeRegistryOwnable().catch((error) => {
  console.error(error);
  process.exit(1);
});
