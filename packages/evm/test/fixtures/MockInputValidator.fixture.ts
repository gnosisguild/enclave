// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ethers } from "hardhat";

import { MockInputValidator__factory } from "../../types/factories/contracts/test/MockInputValidator__factory";

export async function deployInputValidatorFixture() {
  const deployment = await (
    await ethers.getContractFactory("MockInputValidator")
  ).deploy();
  return MockInputValidator__factory.connect(await deployment.getAddress());
}
