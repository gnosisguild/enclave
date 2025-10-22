// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import fs from "fs";
import path from "path";

export const deploymentsFile = path.join("deployed_contracts.json");

// Type for deployment arguments
export interface DeploymentArgs {
  address: string;
  constructorArgs?: Record<string, string | string[]>;
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
    } catch (error) {
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
  } catch (error) {
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
