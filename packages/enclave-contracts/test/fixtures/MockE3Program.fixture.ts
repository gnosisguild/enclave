// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

const { ethers } = await network.connect();

import { MockE3Program__factory } from "../../types/ethers-contracts";

export async function deployE3ProgramFixture(inputValidator: string) {
  const deployment = await (
    await ethers.getContractFactory("MockE3Program")
  ).deploy(inputValidator);
  return MockE3Program__factory.connect(await deployment.getAddress());
}
