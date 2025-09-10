// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ciphernodeAdd } from "@enclave-e3/contracts/tasks/ciphernode";

import hardhatEthersChaiMatchers from "@nomicfoundation/hardhat-ethers-chai-matchers";
import hardhatIgnitionEthers from "@nomicfoundation/hardhat-ignition-ethers";
import hardhatNetworkHelpers from "@nomicfoundation/hardhat-network-helpers";
import hardhatToolboxMochaEthersPlugin from "@nomicfoundation/hardhat-toolbox-mocha-ethers";
import hardhatTypechainPlugin from "@nomicfoundation/hardhat-typechain";

import type { HardhatUserConfig } from "hardhat/config";

const config: HardhatUserConfig = {
  tasks: [
    ciphernodeAdd,
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
  networks: {
    hardhat: {
      type: "edr-simulated",
      chainType: "l1",
    },
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
