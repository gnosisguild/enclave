import { task } from "hardhat/config";
import type { TaskArguments } from "hardhat/types";

task("ciphernode:add", "Register a ciphernode to the registry")
  .addParam("ciphernodeAddress", "address of ciphernode to register")
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const registry = await hre.deployments.get("CyphernodeRegistryOwnable");

    const registryContract = await hre.ethers.getContractAt(
      "CyphernodeRegistryOwnable",
      registry.address,
    );

    const tx = await registryContract.addCyphernode(
      taskArguments.ciphernodeAddress,
    );
    await tx.wait();

    console.log(`Ciphernode ${taskArguments.ciphernodeAddress} registered`);
  });

task("ciphernode:remove", "Remove a ciphernode from the registry")
  .addParam("ciphernodeAddress", "address of ciphernode to remove")
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const registry = await hre.deployments.get("CyphernodeRegistryOwnable");

    const registryContract = await hre.ethers.getContractAt(
      "CyphernodeRegistryOwnable",
      registry.address,
    );

    const tx = await registryContract.removeCyphernode(
      taskArguments.ciphernodeAddress,
    );
    await tx.wait();

    console.log(`Ciphernode ${taskArguments.ciphernodeAddress} removed`);
  });
