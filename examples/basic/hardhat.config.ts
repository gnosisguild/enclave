import "@nomicfoundation/hardhat-toolbox";
import "hardhat-deploy";
import type { HardhatUserConfig } from "hardhat/config";
import "@gnosis-guild/enclave/deploy";

const config: HardhatUserConfig = {
  defaultNetwork: "localhost",
  networks: {
    localhost: {
      url: "http://127.0.0.1:8545",
    },
  },
  solidity: {
    version: "0.8.27",
    settings: {
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
  // Point to the contracts in the installed package
  paths: {
    sources: "./node_modules/@gnosis-guild/enclave/contracts",
    artifacts: "./artifacts",
    cache: "./cache",
    tests: "./test",
  },
  typechain: {
    outDir: "types",
    target: "ethers-v6",
  },
};

export default config;
