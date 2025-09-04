// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// scripts/deploy-local.ts
import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";
// Import the deployment logic from your published package
// This assumes your package exports the deployment function
import deployEnclave from "@enclave-e3/contracts/deploy/enclave";

async function main() {
  console.log("🚀 Deploying Enclave protocol locally...");

  // Get hardhat runtime environment
  const hre = require("hardhat");

  // Get deployer account
  const [deployer] = await hre.ethers.getSigners();
  console.log("Deploying with account:", deployer.address);
  console.log(
    "Account balance:",
    hre.ethers.formatEther(
      await hre.ethers.provider.getBalance(deployer.address),
    ),
  );

  try {
    // Execute the deployment
    await deployEnclave(hre);
    console.log("✅ Enclave protocol deployed successfully!");

    // Log deployed contract addresses
    const enclave = await hre.deployments.get("Enclave");
    const registry = await hre.deployments.get("CiphernodeRegistryOwnable");
    const filter = await hre.deployments.get("NaiveRegistryFilter");

    console.log("\n📋 Deployed Contracts:");
    console.log("Enclave:", enclave.address);
    console.log("CiphernodeRegistryOwnable:", registry.address);
    console.log("NaiveRegistryFilter:", filter.address);
  } catch (error) {
    console.error("❌ Deployment failed:", error);
    process.exit(1);
  }
}

// Execute the deployment
if (require.main === module) {
  main()
    .then(() => process.exit(0))
    .catch((error) => {
      console.error(error);
      process.exit(1);
    });
}
