// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

const { ethers } = await network.connect();

import { MockInputValidator__factory } from "../../types/ethers-contracts";

export async function deployInputValidatorFixture() {
  const deployment = await (
    await ethers.getContractFactory("MockInputValidator")
  ).deploy();
  return MockInputValidator__factory.connect(await deployment.getAddress());
}
