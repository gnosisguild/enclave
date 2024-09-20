import { task } from "hardhat/config";
import type { TaskArguments } from "hardhat/types";

task("ciphernode:add", "Register a ciphernode to the registry")
  .addParam("ciphernodeAddress", "address of ciphernode to register")
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const registry = await hre.deployments.get("CiphernodeRegistryOwnable");

    const registryContract = await hre.ethers.getContractAt(
      "CiphernodeRegistryOwnable",
      registry.address,
    );

    const tx = await registryContract.addCiphernode(
      taskArguments.ciphernodeAddress,
    );
    await tx.wait();

    console.log(`Ciphernode ${taskArguments.ciphernodeAddress} registered`);
  });
