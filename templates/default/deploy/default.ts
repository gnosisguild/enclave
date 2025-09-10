// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { deployAndSaveEnclave } from "@enclave-e3/contracts/scripts/deployAndSave/enclave.js";
import { HardhatRuntimeEnvironment } from "hardhat/types/hre";

export const deployTemplate = async (hre: HardhatRuntimeEnvironment) => {
  // const { ethers  } = await hre.network.connect();
 
  // const { enclave } = await deployAndSaveEnclave({ hre });

  // const verifier = await ethers.deployContract("MockRISC0Verifier");

  // const imageId = await ethers.deployContract("ImageID");

  // const programId = await imageId.PROGRAM_ID();

  // const inputValidator = await ethers.deployContract("InputValidator");

  // const e3Program = await ethers.deployContract("MyProgram", [await enclave.getAddress(), await verifier.getAddress(), programId, await inputValidator.getAddress()]);

  // const tx = await enclave.enableE3Program(await e3Program.getAddress());

  // await tx.wait();
};
