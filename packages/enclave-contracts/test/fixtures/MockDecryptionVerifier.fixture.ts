// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

import { MockDecryptionVerifier__factory } from "../../types";
import MockDecryptionVerifierModule from "../../ignition/modules/mockDecryptionVerifier";

const { ignition } = await network.connect();

export async function deployDecryptionVerifierFixture() {
  const { mockDecryptionVerifier } = await ignition.deploy(MockDecryptionVerifierModule);
  return MockDecryptionVerifier__factory.connect(await mockDecryptionVerifier.getAddress());
}
