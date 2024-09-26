import fs from "fs";
import { task, types } from "hardhat/config";
import type { TaskArguments } from "hardhat/types";

task(
  "committee:new",
  "Request a new ciphernode committee, will use E3 mock contracts by default",
)
  .addOptionalParam(
    "filter",
    "address of filter contract to use",
    undefined,
    types.string,
  )
  .addOptionalParam(
    "thresholdQuorum",
    "threshold quorum for committee",
    2,
    types.int,
  )
  .addOptionalParam(
    "thresholdTotal",
    "threshold total for committee",
    2,
    types.int,
  )
  .addOptionalParam(
    "windowStart",
    "timestamp start of window for the E3 (default: now)",
    Math.floor(Date.now() / 1000),
    types.int,
  )
  .addOptionalParam(
    "windowEnd",
    "timestamp end of window for the E3 (default: now + 1 day)",
    Math.floor(Date.now() / 1000) + 86400,
    types.int,
  )
  .addOptionalParam(
    "duration",
    "duration in seconds of the E3 (default: 1 day)",
    86400,
    types.int,
  )
  .addOptionalParam(
    "e3Address",
    "address of the E3 program",
    undefined,
    types.string,
  )
  .addOptionalParam(
    "e3Params",
    "parameters for the E3 program",
    undefined,
    types.string,
  )
  .addOptionalParam(
    "computeParams",
    "parameters for the compute provider",
    undefined,
    types.string,
  )
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const enclave = await hre.deployments.get("Enclave");

    const enclaveContract = await hre.ethers.getContractAt(
      "Enclave",
      enclave.address,
    );

    let e3Address = taskArguments.e3Address;
    if (!e3Address) {
      const mockE3Program = await hre.deployments.get("MockE3Program");
      if (!mockE3Program) {
        throw new Error("MockE3Program not deployed");
      }
      e3Address = mockE3Program.address;
    }

    let filterAddress = taskArguments.filter;
    if (!filterAddress) {
      const naiveRegistryFilter = await hre.deployments.get(
        "NaiveRegistryFilter",
      );
      if (!naiveRegistryFilter) {
        throw new Error("NaiveRegistryFilter not deployed");
      }
      filterAddress = naiveRegistryFilter.address;
    }

    let e3Params = taskArguments.e3Params;
    if (!e3Params) {
      const MockInputValidator =
        await hre.deployments.get("MockInputValidator");
      if (!MockInputValidator) {
        throw new Error("MockInputValidator not deployed");
      }
      e3Params = hre.ethers.zeroPadValue(MockInputValidator.address, 32);
    }

    let computeParams = taskArguments.computeParams;
    if (!computeParams) {
      // no compute params provided, use mock
      const MockDecryptionVerifier = await hre.deployments.get(
        "MockDecryptionVerifier",
      );
      if (!MockDecryptionVerifier) {
        throw new Error("MockDecryptionVerifier not deployed");
      }
      computeParams = hre.ethers.zeroPadValue(
        MockDecryptionVerifier.address,
        32,
      );
    }

    try {
      const enableE3Tx = await enclaveContract.enableE3Program(e3Address);
      await enableE3Tx.wait();
    } catch (e) {
      console.log("E3 program enabling failed, probably already enabled: ", e);
    }

    const tx = await enclaveContract.request(
      filterAddress,
      [taskArguments.thresholdQuorum, taskArguments.thresholdTotal],
      [taskArguments.windowStart, taskArguments.windowEnd],
      taskArguments.duration,
      e3Address,
      e3Params,
      computeParams,
      // 1 ETH
      { value: "1000000000000000000" },
    );

    console.log("Reequesting committee... ", tx.hash);
    await tx.wait();

    console.log(`Committee requested`);
  });

task("committee:publish", "Publish the publickey of the committee")
  .addOptionalParam(
    "filter",
    "address of filter contract to use (defaults to NaiveRegistryFilter)",
  )
  .addParam("e3Id", "Id of the E3 program", undefined, types.int)
  .addParam(
    "nodes",
    "list of node address in the committee, comma separated",
    undefined,
    types.string,
  )
  .addParam("publicKey", "public key of the committee", undefined, types.string)
  .setAction(async function (taskArguments: TaskArguments, hre) {
    let filterAddress = taskArguments.filter;
    if (!taskArguments.filter) {
      filterAddress = (await hre.deployments.get("NaiveRegistryFilter"))
        .address;

      if (!filterAddress) {
        throw new Error("NaiveRegistryFilter not deployed");
      }
    }

    const filterContract = await hre.ethers.getContractAt(
      "NaiveRegistryFilter",
      filterAddress,
    );

    const nodes = taskArguments.nodes.split(",");

    if (!Array.isArray(nodes)) {
      throw new Error(
        "Could not parse nodes: Nodes must be input as comma separated list",
      );
    }

    const tx = await filterContract.publishCommittee(
      taskArguments.e3Id,
      nodes,
      taskArguments.publicKey,
    );

    console.log("Publishing committee... ", tx.hash);
    await tx.wait();
    console.log(`Committee public key published`);
  });

task("e3:activate", "Activate an E3 program")
  .addParam("e3Id", "Id of the E3 program")
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const enclave = await hre.deployments.get("Enclave");

    const enclaveContract = await hre.ethers.getContractAt(
      "Enclave",
      enclave.address,
    );

    const tx = await enclaveContract.activate(taskArguments.e3Id);

    console.log("Activating E3 program... ", tx.hash);
    await tx.wait();

    console.log(`E3 program activated`);
  });

task("e3:publishInput", "Publish input for an E3 program")
  .addParam("e3Id", "Id of the E3 program")
  .addOptionalParam("data", "data to publish")
  .addOptionalParam("dataFile", "file containing data to publish")
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const enclave = await hre.deployments.get("Enclave");

    const enclaveContract = await hre.ethers.getContractAt(
      "Enclave",
      enclave.address,
    );

    let data = taskArguments.data;

    if (taskArguments.dataFile) {
      const file = fs.readFileSync(taskArguments.dataFile);
      data = file.toString();
    }

    const tx = await enclaveContract.publishInput(taskArguments.e3Id, data);

    console.log("Publishing input... ", tx.hash);
    await tx.wait();

    console.log(`Input published`);
  });

task("e3:publishCiphertext", "Publish ciphertext output for an E3 program")
  .addParam("e3Id", "Id of the E3 program")
  .addOptionalParam("data", "data to publish")
  .addOptionalParam("dataFile", "file containing data to publish")
  .addOptionalParam("proof", "proof to publish")
  .addOptionalParam("proofFile", "file containing proof to publish")
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const enclave = await hre.deployments.get("Enclave");

    const enclaveContract = await hre.ethers.getContractAt(
      "Enclave",
      enclave.address,
    );

    let data = taskArguments.data;

    if (taskArguments.dataFile) {
      const file = fs.readFileSync(taskArguments.dataFile);
      data = "0x" + file.toString("hex");
    }

    let proof = taskArguments.proof;

    if (taskArguments.proofFile) {
      const file = fs.readFileSync(taskArguments.proofFile);
      proof = file.toString();
    }

    const tx = await enclaveContract.publishCiphertextOutput(
      taskArguments.e3Id,
      data,
      proof,
    );

    console.log("Publishing ciphertext... ", tx.hash);
    await tx.wait();

    console.log(`Ciphertext published`);
  });

task("e3:publishPlaintext", "Publish plaintext output for an E3 program")
  .addParam("e3Id", "Id of the E3 program")
  .addOptionalParam("data", "data to publish")
  .addOptionalParam("dataFile", "file containing data to publish")
  .addOptionalParam("proof", "proof to publish")
  .addOptionalParam("proofFile", "file containing proof to publish")
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const enclave = await hre.deployments.get("Enclave");

    const enclaveContract = await hre.ethers.getContractAt(
      "Enclave",
      enclave.address,
    );

    let data = taskArguments.data;

    if (taskArguments.dataFile) {
      const file = fs.readFileSync(taskArguments.dataFile);
      data = file.toString();
    }

    let proof = taskArguments.proof;

    if (taskArguments.proofFile) {
      const file = fs.readFileSync(taskArguments.proofFile);
      proof = file.toString();
    }

    const tx = await enclaveContract.publishPlaintextOutput(
      taskArguments.e3Id,
      data,
      proof,
    );

    console.log("Publishing ciphertext... ", tx.hash);
    await tx.wait();

    console.log(`Ciphertext published`);
  });
