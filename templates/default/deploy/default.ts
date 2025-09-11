// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { readDeploymentArgs, storeDeploymentArgs } from "@enclave-e3/contracts/scripts/utils.js";
import { Enclave__factory as EnclaveFactory } from "@enclave-e3/contracts/types";
import hre from "hardhat";

export const deployTemplate = async () => {
  const { ethers } = await hre.network.connect();
  const [owner] = await ethers.getSigners();

  const chain = hre.globalOptions.network;
 
  const enclaveAddress = readDeploymentArgs("Enclave", chain)?.address;
  if (!enclaveAddress) {
    throw new Error("Enclave address not found, it must be deployed first");
  }
  const enclave = EnclaveFactory.connect(enclaveAddress, owner);
  const verifier = await ethers.deployContract("MockRISC0Verifier");
  await verifier.waitForDeployment();

  storeDeploymentArgs({
    address: await verifier.getAddress(),
  }, "MockRISC0Verifier", chain);

  const imageId = await ethers.deployContract("ImageID");
  await imageId.waitForDeployment();

  storeDeploymentArgs({
    address: await imageId.getAddress(),
  }, "ImageID", chain);

  const programId = await imageId.PROGRAM_ID();

  const inputValidator = await ethers.deployContract("InputValidator");
  await inputValidator.waitForDeployment();

  storeDeploymentArgs({
    address: await inputValidator.getAddress(),
  }, "InputValidator", chain);

  const e3Program = await ethers.deployContract("MyProgram", [await enclave.getAddress(), await verifier.getAddress(), programId, await inputValidator.getAddress()]);
  await e3Program.waitForDeployment();

  storeDeploymentArgs({
    address: await e3Program.getAddress(),
    constructorArgs: {
      enclave: await enclave.getAddress(),
      verifier: await verifier.getAddress(),
      programId,
      inputValidator: await inputValidator.getAddress(),
    },
  }, "MyProgram", chain);

  const tx = await enclave.enableE3Program(await e3Program.getAddress());

  await tx.wait();

  console.log("E3 Program enabled for Enclave's template");
};
