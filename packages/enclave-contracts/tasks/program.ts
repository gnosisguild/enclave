import { task } from "hardhat/config";
import { ArgumentType } from "hardhat/types/arguments";
import { MockE3Program__factory as E3ProgramFactory } from "../types";

import fs from "fs";

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
    // MockProgram
    .addOption({
        name: "programAddress",
        description: "Address of the E3 program",
        defaultValue: "0x7a2088a1bFc9d81c55368AE168C2C02570cB814F",
        type: ArgumentType.STRING,
    })
    .setAction(async () => ({
      default: async ({ e3Id, data, dataFile, programAddress }, hre) => {
        const { deployAndSaveMockProgram } = await import("../scripts/deployAndSave/mockProgram");

        const { ethers } = await hre.network.connect();
        const [signer] = await ethers.getSigners();

        let actualProgramAddress = programAddress;
        if (programAddress === "") {
            actualProgramAddress = await deployAndSaveMockProgram({ hre }).then(({ e3Program }) => e3Program.getAddress());
        }
  
        const program = E3ProgramFactory.connect(
            actualProgramAddress,
            signer,
        )

        let dataToSend = data;
  
        if (dataFile) {
          const file = fs.readFileSync(dataFile);
          dataToSend = file.toString();
        }

        await program.publishInput(e3Id, dataToSend);
    
        console.log(`Input published`);
      },
    }))
    .build();
