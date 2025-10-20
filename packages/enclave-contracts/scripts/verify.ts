// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { execSync } from "child_process";
import fs from "fs";
import path from "path";

import { readAllDeployments } from "./utils";

/**
 * Find the fully qualified contract name by searching the contracts directory
 * @param contractName - Simple contract name (e.g., "Enclave")
 * @param contractsDir - Directory to search (default: "contracts")
 * @returns Fully qualified name or undefined if not found
 */
const findContractPath = (
  contractName: string,
  contractsDir: string = "contracts",
): string | undefined => {
  const searchDir = (dir: string): string | undefined => {
    const files = fs.readdirSync(dir);

    for (const file of files) {
      const fullPath = path.join(dir, file);
      const stat = fs.statSync(fullPath);

      if (stat.isDirectory()) {
        const result = searchDir(fullPath);
        if (result) return result;
      } else if (file.endsWith(".sol")) {
        const content = fs.readFileSync(fullPath, "utf-8");
        // Look for contract definition
        const contractRegex = new RegExp(
          `contract\\s+${contractName}\\s+`,
          "m",
        );
        if (contractRegex.test(content)) {
          // Return in Hardhat's format: relative/path/File.sol:ContractName
          return `${fullPath}:${contractName}`;
        }
      }
    }
    return undefined;
  };

  return searchDir(contractsDir);
};

/**
 * Verify a single contract using Hardhat CLI
 * @param address - Contract address
 * @param constructorArgs - Constructor arguments as a record
 * @param network - Network name
 */
const verifyContract = (
  contractName: string,
  address: string,
  constructorArgs: Record<string, string | string[]> | undefined,
  network: string,
): void => {
  // Create a temporary args file
  const argsFile = path.join(process.cwd(), `verify-args-${address}.cjs`);

  try {
    if (constructorArgs) {
      const argsArray = Object.values(constructorArgs);

      const fileContent = `module.exports = ${JSON.stringify(argsArray, null, 2)};`;
      fs.writeFileSync(argsFile, fileContent);

      const command = `pnpm hardhat verify --build-profile default --network ${network} --contract ${contractName} ${address} --constructor-args-path ${argsFile}`;

      console.log(`Executing: ${command}`);
      execSync(command, { stdio: "inherit" });
      console.log(`✅ Contract verified successfully at ${address}\n`);
    } else {
      const command = `pnpm hardhat verify --build-profile default --network ${network} --contract ${contractName} ${address}`;
      execSync(command, { stdio: "inherit" });
    }
  } catch (error: unknown) {
    if ((error as Error).message?.includes("Already Verified")) {
      console.log(`ℹ️  Contract at ${address} is already verified\n`);
    } else {
      console.error(
        `❌ Failed to verify contract at ${address}:`,
        (error as Error).message,
        "\n",
      );
    }
  } finally {
    // ensure that we always clean up even if there was some early failure
    if (fs.existsSync(argsFile)) {
      fs.unlinkSync(argsFile);
    }
  }
};

/**
 * Deploy and verify all contracts on a given chain
 * @param chain - The chain to verify the contracts on
 */
export const verifyContracts = (chain: string): void => {
  const deployments = readAllDeployments();
  const chainDeployments = deployments[chain];

  if (!chainDeployments) {
    console.log(`❌ No deployments found for chain: ${chain}`);
    return;
  }

  const contractNames = Object.keys(chainDeployments);

  console.log(
    `\n🔍 Verifying ${contractNames.length} contracts on ${chain}...\n`,
  );

  contractNames.forEach((contractName, index) => {
    // we skip PoseidonT3 as it's a library
    if (contractName === "PoseidonT3") {
      console.log(
        `ℹ️  Skipping verification for library contract: ${contractName}`,
      );
      return;
    }

    const deployment = chainDeployments[contractName];

    // Auto-discover the fully qualified contract name
    const fullyQualifiedName = findContractPath(contractName);

    if (!fullyQualifiedName) {
      console.log(
        `❌ Could not find contract source for ${contractName}. Skipping verification.`,
      );
      return;
    }

    verifyContract(
      fullyQualifiedName,
      deployment.address,
      deployment.constructorArgs,
      chain,
    );

    console.log(
      `[${index + 1}/${contractNames.length}] Verifying ${contractName}...`,
    );
  });

  console.log(`\n✅ Verification process completed for ${chain}.\n`);
};
