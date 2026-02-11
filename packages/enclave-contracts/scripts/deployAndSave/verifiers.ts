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
 * Deploys ZKTranscriptLib library required by BB-generated verifiers.
 * Reuses existing deployment if already deployed on the chain.
 */
const deployZKTranscriptLib = async (
  hre: HardhatRuntimeEnvironment,
  chain: string,
): Promise<string> => {
  const libName = "ZKTranscriptLib";

  // Check if library is already deployed
  const existing = readDeploymentArgs(libName, chain);
  if (existing?.address) {
    console.log(`   ${libName} already deployed at ${existing.address}`);
    return existing.address;
  }

  // Deploy the library
  console.log(`   Deploying ${libName}...`);
  const { ethers } = await hre.network.connect();
  const factory = await ethers.getContractFactory(libName);
  const contract = await factory.deploy();
  await contract.waitForDeployment();

  const address = await contract.getAddress();
  const blockNumber = await ethers.provider.getBlockNumber();

  storeDeploymentArgs({ blockNumber, address }, libName, chain);

  console.log(`   ${libName} deployed to: ${address}`);
  return address;
};

/**
 * Deploys a single verifier contract and saves the deployment record.
 * BB-generated verifiers require ZKTranscriptLib to be linked.
 * Skips deployment if the contract is already deployed on the target chain.
 *
 * Note: The library FQN (fully-qualified name) uses the pattern:
 * "contracts/verifier/<ContractName>.sol:ZKTranscriptLib"
 * If you get linking errors, check the contract's compiled artifact for the exact FQN.
 */
export const deployAndSaveVerifier = async (
  contractName: string,
  hre: HardhatRuntimeEnvironment,
  zkTranscriptLibAddress: string,
): Promise<{ address: string }> => {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  // Check if already deployed
  const existing = readDeploymentArgs(contractName, chain);
  if (existing?.address) {
    console.log(`   ${contractName} already deployed at ${existing.address}`);
    return { address: existing.address };
  }

  // Link ZKTranscriptLib - FQN pattern: "contracts/verifier/<ContractName>.sol:ZKTranscriptLib"
  const libraryFQN = `project/contracts/verifier/${contractName}.sol:ZKTranscriptLib`;
  const libraries = {
    [libraryFQN]: zkTranscriptLibAddress,
  };

  // Deploy the verifier contract with linked library
  const factory = await ethers.getContractFactory(contractName, { libraries });
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
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";
  console.log(`   Deploying to network: ${chain}`);

  if (contractNames.length === 0) {
    console.log(
      "   No verifier contracts found in contracts/verifier/. Skipping.",
    );
    return {};
  }

  console.log(`   Found ${contractNames.length} verifier contract(s)`);

  // Deploy ZKTranscriptLib once, reused by all verifiers
  const zkTranscriptLibAddress = await deployZKTranscriptLib(hre, chain);

  const deployments: VerifierDeployments = {};

  for (const name of contractNames) {
    const { address } = await deployAndSaveVerifier(
      name,
      hre,
      zkTranscriptLibAddress,
    );
    deployments[name] = address;
  }

  return deployments;
};
