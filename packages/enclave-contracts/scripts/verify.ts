import { execSync } from "child_process";
import fs from "fs";
import path from "path";

import { readAllDeployments } from "./utils"

/**
 * Find the fully qualified contract name by searching the contracts directory
 * @param contractName - Simple contract name (e.g., "Enclave")
 * @param contractsDir - Directory to search (default: "contracts")
 * @returns Fully qualified name or undefined if not found
 */
const findContractPath = (
    contractName: string,
    contractsDir: string = "contracts"
  ): string | undefined => {
    const searchDir = (dir: string): string | undefined => {
      const files = fs.readdirSync(dir);
      
      for (const file of files) {
        const fullPath = path.join(dir, file);
        const stat = fs.statSync(fullPath);
        
        if (stat.isDirectory()) {
          const result = searchDir(fullPath);
          if (result) return result;
        } else if (file.endsWith('.sol')) {
          const content = fs.readFileSync(fullPath, 'utf-8');
          // Look for contract definition
          const contractRegex = new RegExp(`contract\\s+${contractName}\\s+`, 'm');
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
    try {
      // Convert constructor args to command line arguments
      let argsString = "";
      if (constructorArgs) {
        const argsArray = Object.values(constructorArgs);
        argsString = argsArray.map(arg => {
          // Check if the arg is an array
          if (Array.isArray(arg)) {
            // Convert array to JSON string format for CLI
            return `'${JSON.stringify(arg)}'`;
          }
          // Regular string argument
          return `"${arg}"`;
        }).join(" ");
      }

      const command = `pnpm hardhat verify --network ${network} --contract ${contractName} ${address} ${argsString ? ` ${argsString}` : ""}`;
      
      console.log(`Executing: ${command}`);
      execSync(command, { stdio: "inherit" });
      console.log(`✅ Contract verified successfully at ${address}\n`);
    } catch (error: any) {
      if (error.message?.includes("Already Verified") || error.stdout?.includes("Already Verified")) {
        console.log(`ℹ️  Contract at ${address} is already verified\n`);
      } else {
        console.error(`❌ Failed to verify contract at ${address}:`, error.message, "\n");
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

    console.log(`\n🔍 Verifying ${contractNames.length} contracts on ${chain}...\n`);

    contractNames.forEach((contractName, index) => {
      // we skip PoseidonT3 as it's a library
      if (contractName === "PoseidonT3") {
        return;
      }

      const deployment = chainDeployments[contractName];

      // Auto-discover the fully qualified contract name
      const fullyQualifiedName = findContractPath(contractName);

      if (!fullyQualifiedName) {
          console.log(`❌ Could not find contract source for ${contractName}. Skipping verification.`);
          return;
      }

      verifyContract(fullyQualifiedName, deployment.address, deployment.constructorArgs, chain);

      console.log(`[${index + 1}/${contractNames.length}] Verifying ${contractName}...`);
    })

    console.log(`\n✅ Verification process completed for ${chain}.\n`);
}
