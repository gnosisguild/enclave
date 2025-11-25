// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import fs from "fs";
import yaml from "js-yaml";
import path from "path";

export const deploymentsFile = path.join("deployed_contracts.json");

// Type for deployment arguments
export interface DeploymentArgs {
  address: string;
  constructorArgs?: Record<string, string | string[]>;
  proxyRecords?: Record<string, string | string[]>;
  blockNumber?: number | null;
}

// Type for chain-specific deployments
export interface ChainDeployments {
  [contractName: string]: DeploymentArgs;
}

// Type for the deployments object organized by chain
export interface Deployments {
  [chainName: string]: ChainDeployments;
}

/**
 * Defines the Enclave.config.yaml structure
 */
export interface EnclaveConfig {
  chains: Array<{
    name: string;
    rpc_url: string;
    contracts: {
      e3_program?: { address: string; deploy_block: number };
      enclave?: { address: string; deploy_block: number };
      ciphernode_registry?: { address: string; deploy_block: number };
      bonding_registry?: { address: string; deploy_block: number };
      fee_token?: { address: string; deploy_block: number };
    };
  }>;
  // we don't care about the below fields
  program: unknown;
  nodes: unknown;
}

/**
 * Store the deployment arguments for a given contract and chain
 * @param args - The deployment arguments to store
 * @param contractName - The name of the contract to store the deployments for
 * @param chain - The chain to store the deployments for
 */
export const storeDeploymentArgs = (
  args: DeploymentArgs,
  contractName: string,
  chain: string,
): void => {
  let deployments: Deployments = {};

  // Read existing deployments if file exists
  if (fs.existsSync(deploymentsFile)) {
    try {
      deployments = JSON.parse(
        fs.readFileSync(deploymentsFile, "utf8"),
      ) as Deployments;
    } catch {
      console.warn("Failed to parse existing deployments file, starting fresh");
      deployments = {};
    }
  } else {
    // create a new file
    deployments = {};
    fs.writeFileSync(deploymentsFile, JSON.stringify(deployments, null, 2));
  }

  // Initialize chain if it doesn't exist
  if (!deployments[chain]) {
    deployments[chain] = {};
  }

  // Add or update the contract deployment for the specific chain
  deployments[chain][contractName] = args;

  fs.writeFileSync(deploymentsFile, JSON.stringify(deployments, null, 2));
};

/**
 * Read the deployment arguments for a given contract and chain
 * @param contractName - The name of the contract to read the deployments from
 * @param chain - The chain to read the deployments from
 * @returns The deployment arguments for the given contract and chain
 */
export const readDeploymentArgs = (
  contractName: string,
  chain: string,
): DeploymentArgs | undefined => {
  if (!fs.existsSync(deploymentsFile)) {
    // create a new file
    fs.writeFileSync(deploymentsFile, JSON.stringify({}, null, 2));
    return undefined;
  }

  const deployments = JSON.parse(
    fs.readFileSync(deploymentsFile, "utf8"),
  ) as Deployments;
  return deployments[chain]?.[contractName];
};

/**
 * Read all the deployments from the deployments file
 * @returns All the deployments from the deployments file
 */
export const readAllDeployments = (): Deployments => {
  if (!fs.existsSync(deploymentsFile)) {
    return {};
  }

  try {
    return JSON.parse(fs.readFileSync(deploymentsFile, "utf8")) as Deployments;
  } catch {
    console.warn("Failed to parse deployments file");
    return {};
  }
};

/**
 * Clean the deployments for a given network
 * @param network - The network for which to clean the deployments
 */
export const cleanDeployments = (network: string): void => {
  if (!fs.existsSync(deploymentsFile)) {
    return;
  }

  const deployments = readAllDeployments();
  if (deployments[network]) {
    delete deployments[network];
  }
  fs.writeFileSync(deploymentsFile, JSON.stringify(deployments, null, 2));
};

/**
 * Check if two arrays are equal by checking the values inside
 * @param arr1 - The first array
 * @param arr2 - The second array to check
 * @returns Whether the two arrays are equal
 */
export function areArraysEqual<T>(arr1: T[], arr2: T[]): boolean {
  if (arr1.length !== arr2.length) {
    return false;
  }

  for (let i = 0; i < arr1.length; i++) {
    if (arr1[i] !== arr2[i]) {
      return false;
    }
  }

  return true;
}

/**
 * The function to update the enclave.config.yaml file with the deployed contract addresses
 * @param chainToConfig - The chain name to update in the config
 * @param pathToConfigFile - The path to the enclave.config.yaml file
 * @param contractMapping - A mapping of contract names to config keys
 */
export const updateE3Config = (
  chainToConfig: string,
  pathToConfigFile: string,
  contractMapping: Record<string, string>,
  rpcUrl?: string,
): void => {
  const fileContent = fs.readFileSync(pathToConfigFile, "utf8");
  const config = yaml.load(fileContent) as EnclaveConfig;

  // Find the hardhat chain config
  // Find the chain config or create it
  let configChain = config.chains.find((chain) => chain.name === chainToConfig);

  if (!configChain) {
    console.log(
      `Chain "${chainToConfig}" not found in config. Creating new entry...`,
    );

    if (!rpcUrl) {
      console.warn(
        "Warning: No RPC URL provided. You'll need to update it manually in the config.",
      );
    }

    configChain = {
      name: chainToConfig,
      rpc_url: rpcUrl || `ws://localhost:8545`,
      contracts: {},
    };

    config.chains.push(configChain);
    console.log(`✓ Created new chain entry for "${chainToConfig}"`);
  }

  console.log(`\nUpdating contracts for chain: ${chainToConfig}`);

  // Update contract addresses and deploy blocks
  for (const [contractName, configKey] of Object.entries(contractMapping)) {
    const deployment = readDeploymentArgs(contractName, chainToConfig);

    if (deployment) {
      configChain.contracts[configKey as keyof typeof configChain.contracts] = {
        address: deployment.address,
        deploy_block: deployment.blockNumber ?? 1,
      };
      console.log(
        `✓ Updated ${configKey}: ${deployment.address} (block ${deployment.blockNumber ?? 1})`,
      );
    }
  }

  // Write updated config back to file
  const yamlStr = yaml.dump(config, {
    indent: 2,
    lineWidth: -1, // Don't wrap lines
  });

  fs.writeFileSync(pathToConfigFile, yamlStr + "\n", "utf8");
  console.log("\n✓ enclave.config.yaml updated successfully!");
};
