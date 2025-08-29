// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

import { MockCiphernodeRegistry__factory } from "../../types";
import MockCiphernodeRegistryModule from "../../ignition/modules/mockCiphernodeRegistry";

const { ethers, ignition } = await network.connect();

export async function deployCiphernodeRegistryFixture() {
  const [signer] = await ethers.getSigners();
  const { mockCiphernodeRegistry } = await ignition.deploy(MockCiphernodeRegistryModule);

  return MockCiphernodeRegistry__factory.connect(
    mockCiphernodeRegistry.target,
    signer.provider,
  );
}
