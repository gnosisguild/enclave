// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ethers } from "hardhat";

import { MockDecryptionVerifier__factory } from "../../types/factories/contracts/test/MockDecryptionVerifier__factory";

export async function deployDecryptionVerifierFixture() {
  const deployment = await (
    await ethers.getContractFactory("MockDecryptionVerifier")
  ).deploy();
  return MockDecryptionVerifier__factory.connect(await deployment.getAddress());
}
