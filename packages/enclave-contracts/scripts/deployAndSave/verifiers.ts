// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import fs from "fs";
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";
import path from "path";

import { readDeploymentArgs, storeDeploymentArgs } from "../utils";

/**
 * Discovers all Solidity verifier contracts in contracts/verifier/ directory.
 * Returns an array of contract names (without .sol extension).
 */
export const discoverVerifierContracts = (): string[] => {
  const verifierDir = path.join(process.cwd(), "contracts/verifier");
  if (!fs.existsSync(verifierDir)) {
    return [];
  }

  return fs
    .readdirSync(verifierDir)
    .filter((f) => f.endsWith(".sol"))
    .map((f) => f.replace(".sol", ""));
};

/**
 * Reads a compiled artifact to extract unlinked library references.
 * Returns a map of fully-qualified library name → library name.
 */
const getRequiredLibraries = async (
  contractName: string,
  hre: HardhatRuntimeEnvironment,
): Promise<string[]> => {
  const artifact = await hre.artifacts.readArtifact(contractName);
  const linkRefs = artifact.bytecode.match(/__\$[a-f0-9]{34}\$__/g);
  if (!linkRefs || linkRefs.length === 0) return [];

  // Extract library names from linkReferences in the artifact
  const libraries: string[] = [];
  const linkReferences = (artifact as any).linkReferences ?? {};
  for (const source of Object.keys(linkReferences)) {
    for (const libName of Object.keys(linkReferences[source])) {
      libraries.push(`${source}:${libName}`);
    }
  }
  return libraries;
};

/**
 * Deploys a single verifier contract and saves the deployment record.
 * Automatically detects and deploys any libraries the verifier depends on.
 * Skips deployment if the contract is already deployed on the target chain.
 */
export const deployAndSaveVerifier = async (
  contractName: string,
  hre: HardhatRuntimeEnvironment,
): Promise<{ address: string }> => {
  const { ethers } = await hre.network.connect();
  const chain = hre.globalOptions.network;

  // Check if already deployed
  const existing = readDeploymentArgs(contractName, chain);
  if (existing?.address) {
    console.log(`   ${contractName} already deployed at ${existing.address}`);
    return { address: existing.address };
  }

  // Detect and deploy required libraries
  const requiredLibs = await getRequiredLibraries(contractName, hre);
  const libraries: Record<string, string> = {};

  for (const fqn of requiredLibs) {
    const libName = fqn.split(":").pop()!;
    const libStorageKey = `${contractName}_${libName}`;

    // Check if library is already deployed
    const existingLib = readDeploymentArgs(libStorageKey, chain);
    if (existingLib?.address) {
      console.log(
        `   ${libName} library already deployed at ${existingLib.address}`,
      );
      libraries[fqn] = existingLib.address;
      continue;
    }

    // Deploy the library
    console.log(`   Deploying library ${libName}...`);
    const libFactory = await ethers.getContractFactory(libName);
    const libContract = await libFactory.deploy();
    await libContract.waitForDeployment();
    const libAddress = await libContract.getAddress();
    const libBlockNumber = await ethers.provider.getBlockNumber();

    storeDeploymentArgs(
      { blockNumber: libBlockNumber, address: libAddress },
      libStorageKey,
      chain,
    );

    console.log(`   ${libName} library deployed to: ${libAddress}`);
    libraries[fqn] = libAddress;
  }

  // Deploy the verifier contract with linked libraries
  const factory =
    Object.keys(libraries).length > 0
      ? await ethers.getContractFactory(contractName, { libraries })
      : await ethers.getContractFactory(contractName);

  const contract = await factory.deploy();
  await contract.waitForDeployment();

  const address = await contract.getAddress();
  const blockNumber = await ethers.provider.getBlockNumber();

  storeDeploymentArgs(
    {
      blockNumber,
      address,
    },
    contractName,
    chain,
  );

  console.log(`   ${contractName} deployed to: ${address}`);
  return { address };
};

export interface VerifierDeployments {
  [contractName: string]: string; // contract name → deployed address
}

/**
 * Deploys all verifier contracts found in contracts/verifier/.
 * Skips any that are already deployed on the target chain.
 *
 * @returns A mapping of contract names to their deployed addresses.
 */
export const deployAndSaveAllVerifiers = async (
  hre: HardhatRuntimeEnvironment,
): Promise<VerifierDeployments> => {
  const contractNames = discoverVerifierContracts();

  if (contractNames.length === 0) {
    console.log(
      "   No verifier contracts found in contracts/verifier/. Skipping.",
    );
    return {};
  }

  console.log(`   Found ${contractNames.length} verifier contract(s)`);

  const deployments: VerifierDeployments = {};

  for (const name of contractNames) {
    const { address } = await deployAndSaveVerifier(name, hre);
    deployments[name] = address;
  }

  return deployments;
};
