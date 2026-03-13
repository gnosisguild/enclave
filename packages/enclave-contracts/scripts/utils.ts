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
      slashing_manager?: { address: string; deploy_block: number };
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
 * The function to update the enclave.config.yaml file with the deployed contract addresses.
 * Uses line-by-line text manipulation to preserve comments, blank lines, and quote style.
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
  const content = fs.readFileSync(pathToConfigFile, "utf8");
  const lines = content.split("\n");

  // Collect deployment data keyed by config key
  const updates = new Map<string, { address: string; deployBlock: number }>();
  for (const [contractName, configKey] of Object.entries(contractMapping)) {
    const deployment = readDeploymentArgs(contractName, chainToConfig);
    if (deployment) {
      updates.set(configKey, {
        address: deployment.address,
        deployBlock: deployment.blockNumber ?? 1,
      });
    }
  }

  if (updates.size === 0) {
    console.log("No deployments found to update.");
    return;
  }

  console.log(`\nUpdating contracts for chain: ${chainToConfig}`);

  // State machine to walk through the YAML lines
  let inTargetChain = false;
  let foundTargetChain = false;
  let inContracts = false;
  let currentContractKey: string | null = null;
  let chainBaseIndent = -1;
  let contractsKeyIndent = -1;
  let contractEntryIndent = -1;
  const foundKeys = new Set<string>();
  let lastContractsLine = -1;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    if (trimmed === "" || trimmed.startsWith("#")) {
      if (inContracts) lastContractsLine = i;
      continue;
    }

    const indent = line.length - line.trimStart().length;

    // Detect chain name entry: `  - name: "chainName"`
    const nameMatch = trimmed.match(/^-\s+name:\s*["']?(.+?)["']?\s*$/);
    if (nameMatch) {
      if (inTargetChain) {
        // We've passed the target chain
        break;
      }

      if (nameMatch[1] === chainToConfig) {
        inTargetChain = true;
        foundTargetChain = true;
        chainBaseIndent = indent;
      }
      continue;
    }

    // If we hit a top-level key while in the target chain, we've left it
    if (
      inTargetChain &&
      indent <= chainBaseIndent &&
      !trimmed.startsWith("-")
    ) {
      break;
    }

    if (!inTargetChain) continue;

    // Detect `contracts:` section
    if (trimmed === "contracts:") {
      inContracts = true;
      contractsKeyIndent = indent;
      lastContractsLine = i;
      continue;
    }

    if (!inContracts) continue;

    // Check if we've left the contracts section
    if (indent <= contractsKeyIndent) {
      break;
    }

    lastContractsLine = i;

    // Detect contract key line (e.g., `      enclave:`)
    const keyMatch = trimmed.match(/^(\w+):$/);
    if (
      keyMatch &&
      (contractEntryIndent === -1 || indent === contractEntryIndent)
    ) {
      currentContractKey = keyMatch[1];
      if (contractEntryIndent === -1) contractEntryIndent = indent;
      continue;
    }

    if (!currentContractKey) continue;

    // We're inside a contract entry — update address/deploy_block if this contract needs updating
    const update = updates.get(currentContractKey);
    if (!update) continue;

    if (trimmed.startsWith("address:")) {
      foundKeys.add(currentContractKey);
      const ws = line.match(/^(\s*)/)?.[1] ?? "";
      const comment = trimmed.match(
        /^address:\s*["']?[^#"']*["']?\s*(#.*)$/,
      )?.[1];
      lines[i] =
        `${ws}address: "${update.address}"${comment ? " " + comment : ""}`;
      console.log(
        `✓ Updated ${currentContractKey}: ${update.address} (block ${update.deployBlock})`,
      );
    }

    if (trimmed.startsWith("deploy_block:")) {
      const ws = line.match(/^(\s*)/)?.[1] ?? "";
      const comment = trimmed.match(/^deploy_block:\s*\S+\s*(#.*)$/)?.[1];
      lines[i] =
        `${ws}deploy_block: ${update.deployBlock}${comment ? " " + comment : ""}`;
    }
  }

  if (!foundTargetChain) {
    // Chain not found — append a new chain block at the end of the chains section
    console.log(
      `Chain "${chainToConfig}" not found in config. Creating new entry...`,
    );
    if (!rpcUrl) {
      console.warn(
        "Warning: No RPC URL provided. You'll need to update it manually in the config.",
      );
    }

    const chainsIdx = lines.findIndex((l) => l.trim() === "chains:");
    let insertIdx = lines.length;
    if (chainsIdx !== -1) {
      for (let i = chainsIdx + 1; i < lines.length; i++) {
        const t = lines[i].trim();
        if (t === "" || t.startsWith("#")) continue;
        if (lines[i].length - lines[i].trimStart().length === 0) {
          insertIdx = i;
          break;
        }
      }
    }

    const newLines = [
      `  - name: "${chainToConfig}"`,
      `    rpc_url: "${rpcUrl || "ws://localhost:8545"}"`,
      `    contracts:`,
    ];
    for (const [configKey, update] of updates) {
      newLines.push(`      ${configKey}:`);
      newLines.push(`        address: "${update.address}"`);
      newLines.push(`        deploy_block: ${update.deployBlock}`);
      console.log(
        `✓ Added ${configKey}: ${update.address} (block ${update.deployBlock})`,
      );
    }
    lines.splice(insertIdx, 0, ...newLines);
  } else {
    // Insert any contracts that weren't found in the existing config
    const missingKeys = [...updates.keys()].filter((k) => !foundKeys.has(k));
    if (missingKeys.length > 0 && lastContractsLine !== -1) {
      const keyIndent =
        contractEntryIndent !== -1
          ? contractEntryIndent
          : contractsKeyIndent + 2;
      const valIndent = keyIndent + 2;

      const newLines: string[] = [];
      for (const configKey of missingKeys) {
        const update = updates.get(configKey)!;
        newLines.push(`${" ".repeat(keyIndent)}${configKey}:`);
        newLines.push(`${" ".repeat(valIndent)}address: "${update.address}"`);
        newLines.push(
          `${" ".repeat(valIndent)}deploy_block: ${update.deployBlock}`,
        );
        console.log(
          `✓ Added ${configKey}: ${update.address} (block ${update.deployBlock})`,
        );
      }

      lines.splice(lastContractsLine + 1, 0, ...newLines);
    }
  }

  fs.writeFileSync(pathToConfigFile, lines.join("\n"), "utf8");
  console.log("\n✓ enclave.config.yaml updated successfully!");
};
