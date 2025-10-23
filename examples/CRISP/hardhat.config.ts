// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import type { HardhatUserConfig } from "hardhat/config";
import { cleanDeploymentsTask } from "@enclave-e3/contracts/tasks/utils";
import { ciphernodeAdd } from "@enclave-e3/contracts/tasks/ciphernode";
import dotenv from "dotenv";

import hardhatEthersChaiMatchers from "@nomicfoundation/hardhat-ethers-chai-matchers";
import hardhatNetworkHelpers from "@nomicfoundation/hardhat-network-helpers";
import hardhatToolboxMochaEthersPlugin from "@nomicfoundation/hardhat-toolbox-mocha-ethers";
import hardhatTypechainPlugin from "@nomicfoundation/hardhat-typechain";

dotenv.config();

const mnemonic =
  process.env.MNEMONIC ??
  "test test test test test test test test test test test junk";
const privateKey = process.env.PRIVATE_KEY!;
const rpcUrl = process.env.RPC_URL ?? "http://localhost:8545";

const chainIds = {
  "arbitrum-mainnet": 42161,
  avalanche: 43114,
  bsc: 56,
  ganache: 1337,
  hardhat: 31337,
  mainnet: 1,
  "optimism-mainnet": 10,
  "polygon-mainnet": 137,
  "polygon-mumbai": 80001,
  sepolia: 11155111,
  goerli: 5,
};

function getChainConfig(chain: keyof typeof chainIds, apiUrl: string) {
  let accounts: [string] | { count: number; mnemonic: string; path: string };
  if (privateKey) {
    accounts = [privateKey];
  } else {
    accounts = {
      count: 10,
      mnemonic: mnemonic,
      path: "m/44'/60'/0'/0",
    };
  }

  return {
    accounts,
    chainId: chainIds[chain],
    url: rpcUrl,
    type: "http" as const,
    chainType: "l1" as const,
    blockExplorers: {
      etherscan: {
        apiUrl,
      },
    },
  };
}

const config: HardhatUserConfig = {
  plugins: [
    hardhatTypechainPlugin,
    hardhatEthersChaiMatchers,
    hardhatNetworkHelpers,
    hardhatToolboxMochaEthersPlugin,
  ],
  tasks: [cleanDeploymentsTask, ciphernodeAdd],
  networks: {
    hardhat: {
      type: "edr-simulated",
      chainType: "l1",
    },
    ganache: {
      accounts: {
        mnemonic,
      },
      chainId: chainIds.ganache,
      url: "http://localhost:8545",
      type: "http",
    },
    arbitrum: getChainConfig(
      "arbitrum-mainnet",
      process.env.ARBISCAN_API_KEY || ""
    ),
    avalanche: getChainConfig("avalanche", process.env.SNOWTRACE_API_KEY || ""),
    bsc: getChainConfig("bsc", process.env.BSCSCAN_API_KEY || ""),
    mainnet: getChainConfig("mainnet", process.env.ETHERSCAN_API_KEY || ""),
    optimism: getChainConfig(
      "optimism-mainnet",
      process.env.OPTIMISM_API_KEY || ""
    ),
    "polygon-mainnet": getChainConfig(
      "polygon-mainnet",
      process.env.POLYGONSCAN_API_KEY || ""
    ),
    "polygon-mumbai": getChainConfig(
      "polygon-mumbai",
      process.env.POLYGONSCAN_API_KEY || ""
    ),
    sepolia: getChainConfig("sepolia", process.env.ETHERSCAN_API_KEY || ""),
    goerli: getChainConfig("goerli", process.env.ETHERSCAN_API_KEY || ""),
  },
  paths: {
    artifacts: "./artifacts",
    cache: "./cache",
    sources: "./contracts",
    tests: "./tests",
  },
  typechain: {
    outDir: "./types",
    tsNocheck: false,
  },
  solidity: {
    version: "0.8.28",
    npmFilesToBuild: [
      "poseidon-solidity/PoseidonT3.sol",
      "@enclave-e3/contracts/contracts/Enclave.sol",
      "@enclave-e3/contracts/contracts/registry/CiphernodeRegistryOwnable.sol",
      "@enclave-e3/contracts/contracts/registry/BondingRegistry.sol",
      "@enclave-e3/contracts/contracts/sortition/CommitteeSortition.sol",
      "@enclave-e3/contracts/contracts/test/MockInputValidator.sol",
      "@enclave-e3/contracts/contracts/test/MockCiphernodeRegistry.sol",
      "@enclave-e3/contracts/contracts/test/MockComputeProvider.sol",
      "@enclave-e3/contracts/contracts/test/MockDecryptionVerifier.sol",
      "@enclave-e3/contracts/contracts/test/MockE3Program.sol",
    ],
    settings: {
      optimizer: {
        enabled: true,
        runs: 800,
      },
      metadata: {
        bytecodeHash: "none",
      },
    },
  },
};

export default config;
