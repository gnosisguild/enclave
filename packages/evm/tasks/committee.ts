import { task, types } from "hardhat/config";
import type { TaskArguments } from "hardhat/types";

task("committee:new", "Request a new ciphernode committee")
  .addOptionalParam(
    "filter",
    "address of filter contract to use",
    "0x0000000000000000000000000000000000000006",
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
    "0x95E366f13c16976A26339aBe7992a1AB523388f5",
    types.string,
  )
  .addOptionalParam(
    "e3Params",
    "parameters for the E3 program",
    "0x0000000000000000000000009f3ebc4f6be495901a29bba2ae5a45fb870cdc14",
    types.string,
  )
  .addOptionalParam(
    "computeParams",
    "parameters for the compute provider",
    "0x000000000000000000000000404af1c0780a9269e4d3308a0812fb87bf5fc490",
    types.string,
  )
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const enclave = await hre.deployments.get("Enclave");

    const enclaveContract = await hre.ethers.getContractAt(
      "Enclave",
      enclave.address,
    );

    try {
      const enableE3Tx = await enclaveContract.enableE3Program(
        taskArguments.e3Address,
      );
      await enableE3Tx.wait();
    } catch (e: unknown) {
      console.log(
        "E3 program enabling failed, probably already enabled: ",
        e.message,
      );
    }

    console.log(
      "requesting committee...",
      taskArguments.filter,
      [taskArguments.thresholdQuorum, taskArguments.thresholdTotal],
      [taskArguments.windowStart, taskArguments.windowEnd],
      taskArguments.duration,
      taskArguments.e3Address,
      taskArguments.e3Params,
      taskArguments.computeParams,
    );
    const tx = await enclaveContract.request(
      taskArguments.filter,
      [taskArguments.thresholdQuorum, taskArguments.thresholdTotal],
      [taskArguments.windowStart, taskArguments.windowEnd],
      taskArguments.duration,
      taskArguments.e3Address,
      taskArguments.e3Params,
      taskArguments.computeParams,
      // 1 ETH
      { value: "1000000000000000000" },
    );

    console.log("Reequesting committee... ", tx.hash);
    await tx.wait();

    console.log(`Committee requested`);
  });
