// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { readDeploymentArgs, storeDeploymentArgs } from "@enclave-e3/contracts/scripts";
import { Enclave__factory as EnclaveFactory } from "@enclave-e3/contracts/types";
import { MyProgram__factory as MyProgramFactory } from "../types/factories/contracts";
import hre from "hardhat";

export const deployTemplate = async () => {
  const { ethers } = await hre.network.connect()
  const [owner] = await ethers.getSigners()

  const chain = hre.globalOptions.network

  const enclaveAddress = readDeploymentArgs('Enclave', chain)?.address
  if (!enclaveAddress) {
    throw new Error('Enclave address not found, it must be deployed first')
  }
  const enclave = EnclaveFactory.connect(enclaveAddress, owner);

  const poseidonT3Address = readDeploymentArgs("PoseidonT3", chain)?.address;
  if (!poseidonT3Address) {
    throw new Error("PoseidonT3 address not found, it must be deployed first");
  }
  
  const verifier = await ethers.deployContract('MockRISC0Verifier')
  await verifier.waitForDeployment()

  const imageId = await ethers.deployContract("ImageID");
  await imageId.waitForDeployment();

  storeDeploymentArgs({
    address: await imageId.getAddress(),
  }, "ImageID", chain);

  const programId = await imageId.PROGRAM_ID();

  const e3ProgramFactory = await ethers.getContractFactory(
    MyProgramFactory.abi,
    MyProgramFactory.linkBytecode({
      "npm/poseidon-solidity@0.0.5/PoseidonT3.sol:PoseidonT3": poseidonT3Address,
    }),
    owner
  )
  const e3Program = await e3ProgramFactory.deploy(await enclave.getAddress(), await verifier.getAddress(), programId);
  await e3Program.waitForDeployment();

  const tx = await enclave.enableE3Program(await e3Program.getAddress())

  await tx.wait()

  console.log("E3 Program enabled for Enclave's template")

  console.log(
    `
      Deployed MyProgram at address: ${await e3Program.getAddress()}
      Deployed MockRISC0Verifier at address: ${await verifier.getAddress()}
    `,
  )
};
