// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import type { HardhatUserConfig } from "hardhat/config";
import { configVariable } from "hardhat/config";
import hardhatToolboxViemPlugin from "@nomicfoundation/hardhat-toolbox-viem";

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

  let accounts: [ConfigurationVariable] | {  count: number, mnemonic: ConfigurationVariable, path: string } ;
  if (privateKey) {
    accounts = [privateKey];
  } else {
    accounts = { 
      count: 10,
      mnemonic,
      path: "m/44'/60'/0'/0",
     };
  }

  return {
    accounts,
    chainId: chainIds[chain],
    url: jsonRpcUrl,
    type: 'http' as const,
    chainType: "l1" as const,
    blockExporers: {
      etherscan: {
        apiUrl, 
      },
    },
  };
}

const config: HardhatUserConfig = {
  plugins: [hardhatToolboxViemPlugin],
  networks: {
    hardhat: {
      accounts: {
        mnemonic,
      },
      chainId: chainIds.hardhat,
      type: "edr-simulated",
    },
    ganache: {
      accounts: {
        mnemonic,
      },
      chainId: chainIds.ganache,
      url: "http://localhost:8545",
      type: "http",
    },
    arbitrum: getChainConfig("arbitrum-mainnet", process.env.ARBISCAN_API_KEY || ""),
    avalanche: getChainConfig("avalanche", process.env.SNOWTRACE_API_KEY || ""),
    bsc: getChainConfig("bsc", process.env.BSCSCAN_API_KEY || ""),
    mainnet: getChainConfig("mainnet", process.env.ETHERSCAN_API_KEY || ""),
    optimism: getChainConfig("optimism-mainnet", process.env.OPTIMISM_API_KEY || ""),
    "polygon-mainnet": getChainConfig("polygon-mainnet", process.env.POLYGONSCAN_API_KEY || ""),
    "polygon-mumbai": getChainConfig("polygon-mumbai", process.env.POLYGONSCAN_API_KEY || ""),
    sepolia: getChainConfig("sepolia", process.env.ETHERSCAN_API_KEY || ""),
    goerli: getChainConfig("goerli", process.env.ETHERSCAN_API_KEY || ""),
  },
  paths: {
    artifacts: "./artifacts",
    cache: "./cache",
    sources: "./contracts",
    tests: "./tests",
  },
  solidity: {
    version: "0.8.28",
    settings: {
      metadata: {
        // Not including the metadata hash
        // https://github.com/paulrberg/hardhat-template/issues/31
        bytecodeHash: "none",
      },
      // Disable the optimizer when debugging
      // https://hardhat.org/hardhat-network/#solidity-optimizer-support
      optimizer: {
        enabled: true,
        runs: 800,
      },
    },
  },
};

export default config;
