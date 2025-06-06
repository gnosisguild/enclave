import "@nomicfoundation/hardhat-toolbox";
import "hardhat-deploy";
import "@gnosis-guild/enclave/deploy/enclave";
import { task } from "hardhat/config";
import type { TaskArguments } from "hardhat/types";
import type { HardhatUserConfig } from "hardhat/config";

task("ciphernode:add", "Register a ciphernode to the registry")
  .addParam("ciphernodeAddress", "address of ciphernode to register")
  .setAction(async function(taskArguments: TaskArguments, hre) {
    const registry = await hre.deployments.get("CiphernodeRegistryOwnable");

    const [deployer] = await hre.ethers.getSigners();

    const registryContract = new hre.ethers.Contract(
      registry.address,
      registry.abi,
      deployer,
    );

    const tx = await registryContract.addCiphernode(
      taskArguments.ciphernodeAddress,
    );

    await tx.wait();

    console.log(`Ciphernode ${taskArguments.ciphernodeAddress} registered`);
  });

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
