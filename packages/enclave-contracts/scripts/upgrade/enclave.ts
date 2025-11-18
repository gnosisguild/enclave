// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import hre from "hardhat";

import { upgradeAndSaveEnclave } from "../deployAndSave/enclave";
import { readDeploymentArgs } from "../utils";

/**
 * Upgrades the Enclave contract implementation and saves the deployment arguments
 * This keeps the same proxy address, only updates the implementation
 */
export const upgradeEnclave = async () => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const signerAddress = await signer.getAddress();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";
  console.log("Signer:", signerAddress);

  const preDeployedArgs = readDeploymentArgs("Enclave", chain);
  if (!preDeployedArgs?.address) {
    throw new Error("Enclave proxy not found. Deploy first before upgrading.");
  }

  if (!preDeployedArgs?.proxyRecords?.implementationAddress) {
    throw new Error(
      "Existing deployment is not proxy-based. Cannot upgrade non-proxy deployments.",
    );
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
    ownerAddress: signerAddress,
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
