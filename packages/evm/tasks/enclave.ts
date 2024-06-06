import { task } from "hardhat/config";
import type { TaskArguments } from "hardhat/types";

task("task:deployEnclave", "Deploys Enclave contract")
  .addParam("owner", "Account that will own this contract")
  .addParam("registry", "Address of the cyphernode registry")
  .addParam("maxDuration", "The maximum duration of a computation in seconds")
  .setAction(async function (taskArguments: TaskArguments, { ethers }) {
    const signers = await ethers.getSigners();
    const enclaveFactory = await ethers.getContractFactory("Enclave");
    console.log(`Deploying Enclave...`);
    const enclave = await enclaveFactory
      .connect(signers[0])
      .deploy(
        taskArguments.owner,
        taskArguments.registry,
        taskArguments.maxDuration,
      );
    await enclave.waitForDeployment();
    console.log("Enclave deployed to: ", await enclave.getAddress());
  });
