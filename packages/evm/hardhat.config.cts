import "./tasks/accounts";
import "./tasks/ciphernode";
import "./tasks/enclave";
import "@nomicfoundation/hardhat-chai-matchers";
import "@nomicfoundation/hardhat-toolbox";
import dotenv from "dotenv";
import "hardhat-deploy";
import type { HardhatUserConfig } from "hardhat/config";
import { vars } from "hardhat/config";
import type { NetworkUserConfig } from "hardhat/types";

dotenv.config();

const { INFURA_KEY, MNEMONIC, PRIVATE_KEY, ETHERSCAN_API_KEY } = process.env;

if (!INFURA_KEY || !ETHERSCAN_API_KEY) {
  console.warn(
    "Please set the INFURA_KEY, and ETHERSCAN_API_KEY environment variables to deploy and verify contracts",
  );
}

if (!MNEMONIC && !PRIVATE_KEY) {
  console.warn(
    "Please set a mnemonic or private key to deploy contracts. If you set neither, hardhat will use a default mnemonic",
  );
}

// Setting defaults so that tests will run
const mnemonic =
  MNEMONIC || "test test test test test test test test test test test junk";
const infuraApiKey = INFURA_KEY || "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";

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
};

function getChainConfig(chain: keyof typeof chainIds): NetworkUserConfig {
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

  let accounts: [string] | { mnemonic: string };
  if (PRIVATE_KEY) {
    accounts = [PRIVATE_KEY];
  } else {
    accounts = { mnemonic };
  }

  return {
    accounts,
    chainId: chainIds[chain],
    url: jsonRpcUrl,
  };
}

const config: HardhatUserConfig = {
  defaultNetwork: "hardhat",
  namedAccounts: {
    deployer: 0,
  },
  etherscan: {
    apiKey: {
      arbitrumOne: vars.get("ARBISCAN_API_KEY", ""),
      avalanche: vars.get("SNOWTRACE_API_KEY", ""),
      bsc: vars.get("BSCSCAN_API_KEY", ""),
      mainnet: ETHERSCAN_API_KEY || "",
      optimisticEthereum: vars.get("OPTIMISM_API_KEY", ""),
      polygon: vars.get("POLYGONSCAN_API_KEY", ""),
      polygonMumbai: vars.get("POLYGONSCAN_API_KEY", ""),
      sepolia: ETHERSCAN_API_KEY || "",
    },
  },
  gasReporter: {
    currency: "USD",
    enabled: process.env.REPORT_GAS ? true : false,
    excludeContracts: [],
  },
  networks: {
    hardhat: {
      accounts: {
        mnemonic,
      },
      chainId: chainIds.hardhat,
      allowUnlimitedContractSize: true,
    },
    arbitrum: getChainConfig("arbitrum-mainnet"),
    avalanche: getChainConfig("avalanche"),
    bsc: getChainConfig("bsc"),
    mainnet: getChainConfig("mainnet"),
    optimism: getChainConfig("optimism-mainnet"),
    "polygon-mainnet": getChainConfig("polygon-mainnet"),
    "polygon-mumbai": getChainConfig("polygon-mumbai"),
    sepolia: getChainConfig("sepolia"),
  },
  paths: {
    artifacts: "./artifacts",
    cache: "./cache",
    sources: "./contracts",
    tests: "./test",
  },
  solidity: {
    version: "0.8.27",
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
      viaIR: true,
    },
    overrides: {
      "node_modules/poseidon-solidity/PoseidonT3.sol": {
        version: "0.7.0",
        settings: {
          optimizer: {
            enabled: true,
            runs: 2 ** 32 - 1,
          },
        },
      },
    },
  },
  typechain: {
    outDir: "types",
    target: "ethers-v6",
  },
};

export default config;
