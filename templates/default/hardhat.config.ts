// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ciphernodeAdd } from "@enclave-e3/contracts/tasks/ciphernode";
import { cleanDeploymentsTask } from "@enclave-e3/contracts/tasks/utils";

import hardhatEthersChaiMatchers from "@nomicfoundation/hardhat-ethers-chai-matchers";
import hardhatIgnitionEthers from "@nomicfoundation/hardhat-ignition-ethers";
import hardhatNetworkHelpers from "@nomicfoundation/hardhat-network-helpers";
import hardhatToolboxMochaEthersPlugin from "@nomicfoundation/hardhat-toolbox-mocha-ethers";
import hardhatTypechainPlugin from "@nomicfoundation/hardhat-typechain";

import type { HardhatUserConfig } from "hardhat/config";
import { configVariable } from "hardhat/config";
import { ConfigurationVariable } from "hardhat/types/config";

const mnemonic = configVariable("MNEMONIC");
const privateKey = configVariable("PRIVATE_KEY");
const infuraApiKey = configVariable("INFURA_API_KEY");

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
  let jsonRpcUrl: string;
  switch (chain) {
    case "avalanche":
      jsonRpcUrl = "https://api.avax.network/ext/bc/C/rpc";
      break;
    case "bsc":
      jsonRpcUrl = "https://bsc-dataseed1.binance.org";
      break;
    default:
      jsonRpcUrl = "https://" + chain + ".infura.io/v3/" + infuraApiKey;
  }

  let accounts:
    | [ConfigurationVariable]
    | { count: number; mnemonic: ConfigurationVariable; path: string };
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
    url: jsonRpcUrl,
    type: "http" as const,
    chainType: "l1" as const,
    blockExporers: {
      etherscan: {
        apiUrl,
      },
    },
  };
}

const config: HardhatUserConfig = {
  tasks: [
    ciphernodeAdd,
    cleanDeploymentsTask,
  ],
  plugins: [
    hardhatTypechainPlugin,
    hardhatEthersChaiMatchers,
    hardhatIgnitionEthers,
    hardhatNetworkHelpers,
    hardhatToolboxMochaEthersPlugin,
  ],
  typechain: {
    outDir: "./types",
    tsNocheck: false,
  },
  ignition: {
    strategyConfig: {
      create2: {
        salt: "0x0000000000000000000000000000000000000000000000000000000000000000",
      }
    }
  },
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
      process.env.ARBISCAN_API_KEY || "",
    ),
    avalanche: getChainConfig("avalanche", process.env.SNOWTRACE_API_KEY || ""),
    bsc: getChainConfig("bsc", process.env.BSCSCAN_API_KEY || ""),
    mainnet: getChainConfig("mainnet", process.env.ETHERSCAN_API_KEY || ""),
    optimism: getChainConfig(
      "optimism-mainnet",
      process.env.OPTIMISM_API_KEY || "",
    ),
    "polygon-mainnet": getChainConfig(
      "polygon-mainnet",
      process.env.POLYGONSCAN_API_KEY || "",
    ),
    "polygon-mumbai": getChainConfig(
      "polygon-mumbai",
      process.env.POLYGONSCAN_API_KEY || "",
    ),
    sepolia: getChainConfig("sepolia", process.env.ETHERSCAN_API_KEY || ""),
    goerli: getChainConfig("goerli", process.env.ETHERSCAN_API_KEY || ""),
  },
  solidity: {
    npmFilesToBuild: [
      "poseidon-solidity/PoseidonT3.sol", 
      "@enclave-e3/contracts/contracts/Enclave.sol",
      "@enclave-e3/contracts/contracts/registry/CiphernodeRegistryOwnable.sol",
      "@enclave-e3/contracts/contracts/registry/NaiveRegistryFilter.sol",
      "@enclave-e3/contracts/contracts/test/MockInputValidator.sol",
      "@enclave-e3/contracts/contracts/test/MockCiphernodeRegistry.sol",
      "@enclave-e3/contracts/contracts/test/MockComputeProvider.sol",
      "@enclave-e3/contracts/contracts/test/MockDecryptionVerifier.sol",
      "@enclave-e3/contracts/contracts/test/MockE3Program.sol",
      "@enclave-e3/contracts/contracts/test/MockRegistryFilter.sol",
    ],
    compilers: [
      {
        version: "0.8.27",
        settings: {
          optimizer: {
            enabled: true,
            runs: 800,
          },
        },
      },
    ],
  },
};

export default config;
