// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { deployAndSaveEnclave } from "@enclave-e3/contracts/scripts/deployAndSave/enclave.js";
import hre from "hardhat";

export const deployTemplate = async () => {
  const { ethers } = await hre.network.connect();
  const [owner] = await ethers.getSigners();

  const polynomial_degree = ethers.toBigInt(2048);
  const plaintext_modulus = ethers.toBigInt(1032193);
  const moduli = [ethers.toBigInt("18014398492704769")];

  const encoded = ethers.AbiCoder.defaultAbiCoder().encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );

  const THIRTY_DAYS_IN_SECONDS = 60 * 60 * 24 * 30;
  const addressOne = "0x0000000000000000000000000000000000000001";
 
  const { enclave } = await deployAndSaveEnclave({ maxDuration: THIRTY_DAYS_IN_SECONDS.toString(), params: encoded, owner: await owner.getAddress(), registry: addressOne, hre });

  const verifier = await ethers.deployContract("MockRISC0Verifier");

  const imageId = await ethers.deployContract("ImageID");

  const programId = await imageId.PROGRAM_ID();

  const inputValidator = await ethers.deployContract("InputValidator");

  const e3Program = await ethers.deployContract("MyProgram", [await enclave.getAddress(), await verifier.getAddress(), programId, await inputValidator.getAddress()]);

  console.log("e3Program", await e3Program.getAddress());
  const tx = await enclave.enableE3Program(await e3Program.getAddress());

  await tx.wait();
};

deployTemplate().catch(console.error);