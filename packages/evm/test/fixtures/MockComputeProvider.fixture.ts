// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ethers } from "hardhat";

import { MockComputeProvider__factory } from "../../types/factories/contracts/test/MockComputeProvider__factory";

export async function deployComputeProviderFixture() {
  const deployment = await (
    await ethers.getContractFactory("MockComputeProvider")
  ).deploy();

  return MockComputeProvider__factory.connect(await deployment.getAddress());
}
