import type { HardhatUserConfig } from "hardhat/config";
import "@nomicfoundation/hardhat-toolbox";
import "hardhat-deploy";
import "@gnosis-guild/enclave/deploy/enclave";

const config: HardhatUserConfig = {
  solidity: {
    version: "0.8.27",
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
  external: {
    contracts: [
      {
        artifacts: "node_modules/@gnosis-guild/enclave/artifacts",
      },
    ],
  },
  namedAccounts: {
    deployer: {
      default: 0, // Use the first account as deployer
    },
  },
};

export default config;
