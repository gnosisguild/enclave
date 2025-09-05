import fs from "fs";
import path from "path";

const deploymentsFile = path.join("deployed_contracts.json");

// Type for deployment arguments
export interface DeploymentArgs {
  address: string;
  constructorArgs?: Record<string, string>;
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

export const storeDeploymentArgs = (
  args: DeploymentArgs,
  contractName: string,
  chain: string,
) => {
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
  }

  // Initialize chain if it doesn't exist
  if (!deployments[chain]) {
    deployments[chain] = {};
  }

  // Add or update the contract deployment for the specific chain
  deployments[chain][contractName] = args;

  fs.writeFileSync(deploymentsFile, JSON.stringify(deployments, null, 2));
};

export const readDeploymentArgs = (
  contractName: string,
  chain: string,
): DeploymentArgs | undefined => {
  const deployments = JSON.parse(
    fs.readFileSync(deploymentsFile, "utf8"),
  ) as Deployments;
  return deployments[chain]?.[contractName];
};

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
