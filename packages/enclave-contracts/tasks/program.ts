// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import fs from "fs";
import { task } from "hardhat/config";
import { ArgumentType } from "hardhat/types/arguments";

import { readDeploymentArgs } from "../scripts/utils";

export const publishInput = task(
  "e3-program:publishInput",
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
  // MockProgram. Defaults to the address in deployed_contracts.json for the
  // active network; pass --program-address to override.
  .addOption({
    name: "programAddress",
    description:
      "Address of the E3 program (defaults to deployed MockE3Program)",
    defaultValue: "",
    type: ArgumentType.STRING,
  })
  .setAction(async () => ({
    default: async ({ e3Id, data, dataFile, programAddress }, hre) => {
      const { deployAndSaveMockProgram } = await import(
        "../scripts/deployAndSave/mockProgram"
      );
      const { MockE3Program__factory } = await import("../types");

      const { ethers } = await hre.network.connect();
      const [signer] = await ethers.getSigners();

      let actualProgramAddress = programAddress;
      if (!actualProgramAddress) {
        const deployed = readDeploymentArgs(
          "MockE3Program",
          hre.globalOptions.network,
        );
        if (deployed?.address) {
          actualProgramAddress = deployed.address;
        } else {
          actualProgramAddress = await deployAndSaveMockProgram({ hre }).then(
            ({ e3Program }) => e3Program.getAddress(),
          );
        }
      }

      const program = MockE3Program__factory.connect(
        actualProgramAddress,
        signer,
      );

      let dataToSend = data;

      if (dataFile) {
        const file = fs.readFileSync(dataFile);
        // Hex-encode binary file contents so ethers ABI-encodes them as `bytes`.
        dataToSend = "0x" + file.toString("hex");
      }

      await program.publishInput(e3Id, dataToSend);

      console.log(`Input published to ${actualProgramAddress} (e3Id=${e3Id})`);
    },
  }))
  .build();

// Wire MockE3Program → Enclave so `publishInput` forwards to
// `publishCiphertextOutput`. Off by default; the proof-aggregation integration
// flow opts in by calling this once after deploy. The non-aggregation `base`
// flow does NOT wire it, preserving the pre-existing fake_encrypt path which
// posts the ciphertext via `e3:publishCiphertext` directly.
export const setMockProgramEnclave = task(
  "e3-program:setMockEnclave",
  "Wire MockE3Program → Enclave for the proof-aggregation integration test",
)
  .setAction(async () => ({
    default: async (_args, hre) => {
      const { ethers } = await hre.network.connect();
      const [signer] = await ethers.getSigners();
      const network = hre.globalOptions.network;

      const mockArgs = readDeploymentArgs("MockE3Program", network);
      const enclaveArgs = readDeploymentArgs("Enclave", network);
      if (!mockArgs?.address || !enclaveArgs?.address) {
        throw new Error(
          "MockE3Program or Enclave deployment not found; deploy first.",
        );
      }

      // Use ABI fragments directly so this works even when typechain types
      // haven't been regenerated.
      const mockProgram = new ethers.Contract(
        mockArgs.address,
        [
          "function enclave() view returns (address)",
          "function setEnclave(address) external",
        ],
        signer,
      );
      const current: string = await mockProgram.enclave();
      if (current.toLowerCase() === enclaveArgs.address.toLowerCase()) {
        console.log(`MockE3Program already wired to ${enclaveArgs.address}`);
        return;
      }
      await mockProgram.setEnclave(enclaveArgs.address);
      console.log(
        `MockE3Program ${mockArgs.address} → Enclave ${enclaveArgs.address}`,
      );
    },
  }))
  .build();
