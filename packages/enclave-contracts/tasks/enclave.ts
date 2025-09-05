// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { ZeroAddress } from "ethers";
import fs from "fs";
import { task } from "hardhat/config";
import { ArgumentType } from "hardhat/types/arguments";

import EnclaveModule from "../ignition/modules/enclave";
import MockDecryptionVerifierModule from "../ignition/modules/mockDecryptionVerifier";
import MockE3ProgramModule from "../ignition/modules/mockE3Program";
import MockInputValidatorModule from "../ignition/modules/mockInputValidator";
import NaiveRegistryFilterModule from "../ignition/modules/naiveRegistryFilter";

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
  .setAction(async () => ({
    default: async function (
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
      },
      hre,
    ) {
      const { ignition } = await hre.network.connect();

      const { enclave } = await ignition.deploy(EnclaveModule);

      let actualE3Address = e3Address;

      if (!e3Address) {
        const mockE3Program = await ignition.deploy(MockE3ProgramModule);
        if (!mockE3Program) {
          throw new Error("MockE3Program not deployed");
        }
        actualE3Address = await mockE3Program.mockE3Program.getAddress();
      }

      let filterAddress = filter;
      if (!filterAddress) {
        const naiveRegistryFilter = await ignition.deploy(
          NaiveRegistryFilterModule,
        );
        if (!naiveRegistryFilter) {
          throw new Error("NaiveRegistryFilter not deployed");
        }
        filterAddress =
          await naiveRegistryFilter.naiveRegistryFilter.getAddress();
      }

      let e3ParamsToSend = e3Params;
      if (!e3Params) {
        const mockInputValidator = await ignition.deploy(
          MockInputValidatorModule,
        );

        e3ParamsToSend =
          await mockInputValidator.mockInputValidator.getAddress();
      }

      let computeParamsToSend = computeParams;
      if (!computeParams) {
        // no compute params provided, use mock
        const MockDecryptionVerifier = await ignition.deploy(
          MockDecryptionVerifierModule,
        );
        if (!MockDecryptionVerifier) {
          throw new Error("MockDecryptionVerifier not deployed");
        }
        computeParamsToSend =
          await MockDecryptionVerifier.mockDecryptionVerifier.getAddress();
      }

      try {
        const enableE3Tx = await enclave.enableE3Program(e3Address);
        await enableE3Tx.wait();
      } catch (e) {
        console.log(
          "E3 program enabling failed, probably already enabled: ",
          e,
        );
      }

      const tx = await enclave.request(
        filterAddress,
        [thresholdQuorum, thresholdTotal],
        [windowStart, windowEnd],
        duration,
        actualE3Address,
        e3ParamsToSend,
        computeParamsToSend,
        // 1 ETH
        { value: "1000000000000000000" },
      );

      console.log("Reequesting committee... ", tx.hash);
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
    default: async function ({ e3Address }, hre) {
      const { ignition } = await hre.network.connect();

      const enclave = await ignition.deploy(EnclaveModule);

      const tx = await enclave.enclave.enableE3Program(e3Address);

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
    name: "filter",
    description:
      "address of filter contract to use (defaults to NaiveRegistryFilter)",
    defaultValue: ZeroAddress,
    type: ArgumentType.STRING,
  })
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
    default: async function ({ filter, e3Id, nodes, publicKey }, hre) {
      const { ignition } = await hre.network.connect();

      let filterAddress = filter;
      if (!filterAddress) {
        const naiveRegistryFilter = await ignition.deploy(
          NaiveRegistryFilterModule,
        );
        if (!naiveRegistryFilter) {
          throw new Error("NaiveRegistryFilter not deployed");
        }
        filterAddress =
          await naiveRegistryFilter.naiveRegistryFilter.getAddress();
      }

      const nodesToSend = nodes.split(",");

      const filterContract = await ignition.deploy(NaiveRegistryFilterModule);
      if (!filterContract) {
        throw new Error("NaiveRegistryFilter not deployed");
      }

      const tx = await filterContract.naiveRegistryFilter.publishCommittee(
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
    default: async function ({ e3Id, publicKey }, hre) {
      const { ignition } = await hre.network.connect();

      const { enclave } = await ignition.deploy(EnclaveModule);

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
    default: async function ({ e3Id, data, dataFile }, hre) {
      const { ignition } = await hre.network.connect();

      const enclave = await ignition.deploy(EnclaveModule);

      let dataToSend = data;

      if (dataFile) {
        const file = fs.readFileSync(dataFile);
        dataToSend = file.toString();
      }

      const tx = await enclave.enclave.publishInput(e3Id, dataToSend);

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
    default: async function ({ e3Id, data, dataFile, proof, proofFile }, hre) {
      const { ignition } = await hre.network.connect();

      const enclave = await ignition.deploy(EnclaveModule);

      const enclaveContract = await enclave.enclave;

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

      const tx = await enclaveContract.publishCiphertextOutput(
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
    default: async function ({ e3Id, data, dataFile, proof, proofFile }, hre) {
      const { ignition } = await hre.network.connect();

      const enclave = await ignition.deploy(EnclaveModule);

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

      const tx = await enclave.enclave.publishPlaintextOutput(
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
