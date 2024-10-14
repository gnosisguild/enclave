import { LeanIMT } from "@zk-kit/lean-imt";
import { task } from "hardhat/config";
import type { TaskArguments } from "hardhat/types";
import { poseidon2 } from "poseidon-lite";

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

task("ciphernode:remove", "Remove a ciphernode from the registry")
  .addParam("ciphernodeAddress", "address of ciphernode to remove")
  .addParam("siblings", "comma separated siblings from tree proof")
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const registry = await hre.deployments.get("CiphernodeRegistryOwnable");

    const registryContract = await hre.ethers.getContractAt(
      "CiphernodeRegistryOwnable",
      registry.address,
    );

    const siblings = taskArguments.siblings
      .split(",")
      .map((s: string) => BigInt(s));

    const tx = await registryContract.removeCiphernode(
      taskArguments.ciphernodeAddress,
      siblings,
    );
    await tx.wait();

    console.log(`Ciphernode ${taskArguments.ciphernodeAddress} removed`);
  });

task("ciphernode:siblings", "Get the sibling of a ciphernode in the registry")
  .addParam("ciphernodeAddress", "address of ciphernode to get siblings for")
  .addParam(
    "ciphernodeAddresses",
    "comma separated addresses of ciphernodes in the order they were added to the registry",
  )
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const hash = (a: bigint, b: bigint) => poseidon2([a, b]);
    const tree = new LeanIMT(hash);

    const addresses = taskArguments.ciphernodeAddresses.split(",");

    for (const address of addresses) {
      tree.insert(BigInt(address));
    }

    const index = tree.indexOf(BigInt(taskArguments.ciphernodeAddress));
    const { siblings } = tree.generateProof(index);

    console.log(`Siblings for ${taskArguments.ciphernodeAddress}: ${siblings}`);
  });
