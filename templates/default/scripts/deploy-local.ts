// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { deployEnclave } from "@enclave-e3/contracts/deploy/enclave";
import { deployTemplate } from "../deploy/default";

async function main() {
  console.log("🚀 Deploying Enclave protocol locally...");

  // Get hardhat runtime environment
  const hre = await import("hardhat");

  const { ethers } = await hre.network.connect();

  // Get deployer account
  const [deployer] = await ethers.getSigners();
  console.log("Deploying with account:", deployer.address);
  console.log(
    "Account balance:",
    ethers.formatEther(
      await ethers.provider.getBalance(deployer.address),
    ),
  );

  // Execute the deployment
  await deployEnclave(true);
  await deployTemplate();
}

// Execute the deployment
main().catch(console.error);
