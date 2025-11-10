// SPDX-License-Identifier: LGPL-3.0-only
import hre from "hardhat";

import { upgradeAndSaveEnclave } from "../deployAndSave/enclave";
import { deployAndSavePoseidonT3 } from "../deployAndSave/poseidonT3";
import { readDeploymentArgs } from "../utils";

/**
 * Upgrades the Enclave contract implementation and saves the deployment arguments
 * This keeps the same proxy address, only updates the implementation
 */
export const upgradeEnclave = async () => {
  const { ethers } = await hre.network.connect();
  const [owner] = await ethers.getSigners();
  const ownerAddress = await owner.getAddress();
  const chain = hre.globalOptions.network;
  console.log("Owner:", ownerAddress);

  const poseidonT3 = await deployAndSavePoseidonT3({ hre });
  const preDeployedArgs = readDeploymentArgs("Enclave", chain);
  if (!preDeployedArgs?.address) {
    throw new Error("Enclave proxy not found. Deploy first before upgrading.");
  }

  console.log(
    "Enclave Proxy Address (from deployments):",
    preDeployedArgs.address,
  );

  const code = await ethers.provider.getCode(preDeployedArgs.address);
  if (code === "0x") {
    throw new Error(
      `No contract found at proxy address ${preDeployedArgs.address}`,
    );
  }
  console.log("Proxy contract exists on-chain");

  const { enclave, implementationAddress } = await upgradeAndSaveEnclave({
    poseidonT3Address: poseidonT3,
    ownerAddress: ownerAddress,
    hre,
  });

  const enclaveAddress = await enclave.getAddress();

  console.log(`
    ============================================
    Upgrade Complete!
    ============================================
    Proxy Address: ${enclaveAddress}
    New Implementation: ${implementationAddress}
    ============================================
  `);
};

upgradeEnclave().catch((error) => {
  console.error(error);
  process.exit(1);
});
