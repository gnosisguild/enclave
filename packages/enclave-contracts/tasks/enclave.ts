// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { ZeroAddress, zeroPadValue } from "ethers";
import fs from "fs";
import { task } from "hardhat/config";
import { ArgumentType } from "hardhat/types/arguments";

import { readDeploymentArgs } from "../scripts/utils";

export const requestCommittee = task(
  "committee:new",
  "Request a new ciphernode committee, will use E3 mock contracts by default",
)
  .addOption({
    name: "filter",
    description: "address of filter contract to use",
    defaultValue: ZeroAddress,
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "thresholdQuorum",
    description: "threshold quorum for committee",
    defaultValue: 2,
    type: ArgumentType.INT,
  })
  .addOption({
    name: "thresholdTotal",
    description: "threshold total for committee",
    defaultValue: 2,
    type: ArgumentType.INT,
  })
  .addOption({
    name: "windowStart",
    description: "timestamp start of window for the E3 (default: now)",
    defaultValue: Math.floor(Date.now() / 1000),
    type: ArgumentType.INT,
  })
  .addOption({
    name: "windowEnd",
    description: "timestamp end of window for the E3 (default: now + 1 day)",
    defaultValue: Math.floor(Date.now() / 1000) + 86400,
    type: ArgumentType.INT,
  })
  .addOption({
    name: "duration",
    description: "duration in seconds of the E3 (default: 1 day)",
    defaultValue: 86400,
    type: ArgumentType.INT,
  })
  .addOption({
    name: "e3Address",
    description: "address of the E3 program",
    defaultValue: ZeroAddress,
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "e3Params",
    description: "parameters for the E3 program",
    defaultValue: ZeroAddress,
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "computeParams",
    description: "parameters for the compute provider",
    defaultValue: ZeroAddress,
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "customParams",
    description: "parameters for the custom params",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .setAction(async () => ({
    default: async (
      {
        filter,
        thresholdQuorum,
        thresholdTotal,
        windowStart,
        windowEnd,
        duration,
        e3Address,
        e3Params,
        computeParams,
        customParams,
      },
      hre,
    ) => {
      const { deployAndSaveEnclave } = await import(
        "../scripts/deployAndSave/enclave"
      );

      const { enclave } = await deployAndSaveEnclave({ hre });

      const enclaveArgs = readDeploymentArgs(
        "Enclave",
        hre.globalOptions.network,
      );

      if (!enclaveArgs) {
        throw new Error("Enclave deployment arguments not found");
      }

      const registryArgs = readDeploymentArgs(
        "CiphernodeRegistry",
        hre.globalOptions.network,
      );

      if (!registryArgs) {
        throw new Error("CiphernodeRegistry deployment arguments not found");
      }

      const filterArgs = readDeploymentArgs(
        "NaiveRegistryFilter",
        hre.globalOptions.network,
      );

      if (!filterArgs) {
        throw new Error("NaiveRegistryFilter deployment arguments not found");
      }

      const mockE3ProgramArgs = readDeploymentArgs(
        "MockE3Program",
        hre.globalOptions.network,
      );

      let e3ProgramParams = e3Params;

      const mockInputValidatorArgs = readDeploymentArgs(
        "MockInputValidator",
        hre.globalOptions.network,
      );
      if (e3ProgramParams === ZeroAddress) {
        if (!mockInputValidatorArgs) {
          throw new Error("MockInputValidator deployment arguments not found");
        }
        e3ProgramParams = zeroPadValue(mockInputValidatorArgs.address, 32);
      }

      let computeProviderParams = computeParams;
      const mockDecryptionVerifierArgs = readDeploymentArgs(
        "MockDecryptionVerifier",
        hre.globalOptions.network,
      );
      if (computeProviderParams === ZeroAddress) {
        if (!mockDecryptionVerifierArgs) {
          throw new Error(
            "MockDecryptionVerifier deployment arguments not found",
          );
        }
        computeProviderParams = zeroPadValue(
          mockDecryptionVerifierArgs.address,
          32,
        );
      }

      console.log({
        filter: filter === ZeroAddress ? filterArgs.address : filter,
        threshold: [thresholdQuorum, thresholdTotal],
        startWindow: [windowStart, windowEnd],
        duration: duration,
        e3Program:
          e3Address === ZeroAddress ? mockE3ProgramArgs!.address : e3Address,
        e3ProgramParams,
        computeProviderParams,
        customParams,
      });
      const tx = await enclave.request(
        {
          filter: filter === ZeroAddress ? filterArgs.address : filter,
          threshold: [thresholdQuorum, thresholdTotal],
          startWindow: [windowStart, windowEnd],
          duration: duration,
          e3Program:
            e3Address === ZeroAddress ? mockE3ProgramArgs!.address : e3Address,
          e3ProgramParams,
          computeProviderParams,
          customParams,
        },
        { value: "1000000000000000000" },
      );

      console.log("Requesting committee... ", tx.hash);
      await tx.wait();

      console.log(`Committee requested`);
    },
  }))
  .build();

export const enableE3 = task("enclave:enableE3", "Enable an E3 program")
  .addOption({
    name: "e3Address",
    description: "address of the E3 program",
    defaultValue: ZeroAddress,
    type: ArgumentType.STRING,
  })
  .setAction(async () => ({
    default: async ({ e3Address }, hre) => {
      const { deployAndSaveEnclave } = await import(
        "../scripts/deployAndSave/enclave"
      );

      const { enclave } = await deployAndSaveEnclave({ hre });

      const tx = await enclave.enableE3Program(e3Address);

      console.log("Enabling E3 program... ", tx.hash);
      await tx.wait();

      console.log(`E3 program enabled`);
    },
  }))
  .build();

export const publishCommittee = task(
  "committee:publish",
  "Publish the publickey of the committee",
)
  .addOption({
    name: "e3Id",
    description: "Id of the E3 program",
    defaultValue: 0,
    type: ArgumentType.INT,
  })
  .addOption({
    name: "nodes",
    description: "list of node address in the committee, comma separated",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "publicKey",
    description: "public key of the committee",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .setAction(async () => ({
    default: async ({ e3Id, nodes, publicKey }, hre) => {
      const { deployAndSaveNaiveRegistryFilter } = await import(
        "../scripts/deployAndSave/naiveRegistryFilter"
      );

      const { naiveRegistryFilter } = await deployAndSaveNaiveRegistryFilter({
        hre,
      });

      const nodesToSend = nodes
        .split(",")
        .map((node) => node.trim())
        .filter((node) => node.length > 0);

      if (nodesToSend.length === 0 && nodes.length > 0) {
        throw new Error("Invalid nodes format: no valid addresses found");
      }

      const tx = await naiveRegistryFilter.publishCommittee(
        e3Id,
        nodesToSend,
        publicKey,
      );

      console.log("Publishing committee... ", tx.hash);
      await tx.wait();
      console.log(`Committee public key published`);
    },
  }))
  .build();

export const activateE3 = task("e3:activate", "Activate an E3 program")
  .addOption({
    name: "e3Id",
    description: "Id of the E3 program",
    defaultValue: 0,
    type: ArgumentType.INT,
  })
  .addOption({
    name: "publicKey",
    description: "public key of the committee",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .setAction(async () => ({
    default: async ({ e3Id, publicKey }, hre) => {
      const { deployAndSaveEnclave } = await import(
        "../scripts/deployAndSave/enclave"
      );

      const { enclave } = await deployAndSaveEnclave({ hre });

      const tx = await enclave.activate(e3Id, publicKey);

      console.log("Activating E3 program... ", tx.hash);
      await tx.wait();

      console.log(`E3 program activated`);
    },
  }))
  .build();

export const publishInput = task(
  "e3:publishInput",
  "Publish input for an E3 program",
)
  .addOption({
    name: "e3Id",
    description: "Id of the E3 program",
    defaultValue: 0,
    type: ArgumentType.INT,
  })
  .addOption({
    name: "data",
    description: "data to publish",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "dataFile",
    description: "file containing data to publish",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .setAction(async () => ({
    default: async ({ e3Id, data, dataFile }, hre) => {
      const { deployAndSaveEnclave } = await import(
        "../scripts/deployAndSave/enclave"
      );

      const { enclave } = await deployAndSaveEnclave({ hre });

      let dataToSend = data;

      if (dataFile) {
        const file = fs.readFileSync(dataFile);
        dataToSend = file.toString();
      }

      const tx = await enclave.publishInput(e3Id, dataToSend);

      console.log("Publishing input... ", tx.hash);
      await tx.wait();

      console.log(`Input published`);
    },
  }))
  .build();

export const publishCiphertext = task(
  "e3:publishCiphertext",
  "Publish ciphertext output for an E3 program",
)
  .addOption({
    name: "e3Id",
    description: "Id of the E3 program",
    defaultValue: 0,
    type: ArgumentType.INT,
  })
  .addOption({
    name: "data",
    description: "data to publish",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "dataFile",
    description: "file containing data to publish",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "proof",
    description: "proof to publish",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "proofFile",
    description: "file containing proof to publish",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .setAction(async () => ({
    default: async ({ e3Id, data, dataFile, proof, proofFile }, hre) => {
      const { deployAndSaveEnclave } = await import(
        "../scripts/deployAndSave/enclave"
      );

      const { enclave } = await deployAndSaveEnclave({ hre });

      let dataToSend = data;

      if (dataFile) {
        const file = fs.readFileSync(dataFile);
        dataToSend = "0x" + file.toString("hex");
      }

      let proofToSend = proof;

      if (proofFile) {
        const file = fs.readFileSync(proofFile);
        proofToSend = file.toString();
      }

      const tx = await enclave.publishCiphertextOutput(
        e3Id,
        dataToSend,
        proofToSend,
      );

      console.log("Publishing ciphertext... ", tx.hash);
      await tx.wait();

      console.log(`Ciphertext published`);
    },
  }))
  .build();

export const publishPlaintext = task(
  "e3:publishPlaintext",
  "Publish plaintext output for an E3 program",
)
  .addOption({
    name: "e3Id",
    description: "Id of the E3 program",
    defaultValue: 0,
    type: ArgumentType.INT,
  })
  .addOption({
    name: "data",
    description: "data to publish",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "dataFile",
    description: "file containing data to publish",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "proof",
    description: "proof to publish",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .addOption({
    name: "proofFile",
    description: "file containing proof to publish",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .setAction(async () => ({
    default: async ({ e3Id, data, dataFile, proof, proofFile }, hre) => {
      const { deployAndSaveEnclave } = await import(
        "../scripts/deployAndSave/enclave"
      );

      const { enclave } = await deployAndSaveEnclave({ hre });

      let dataToSend = data;

      if (dataFile) {
        const file = fs.readFileSync(dataFile);
        dataToSend = file.toString();
      }

      let proofToSend = proof;

      if (proofFile) {
        const file = fs.readFileSync(proofFile);
        proofToSend = file.toString();
      }

      const tx = await enclave.publishPlaintextOutput(
        e3Id,
        dataToSend,
        proofToSend,
      );

      console.log("Publishing plaintext... ", tx.hash);
      await tx.wait();

      console.log(`Plaintext published`);
    },
  }))
  .build();
