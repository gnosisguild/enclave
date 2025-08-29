// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

import { MockComputeProvider__factory } from "../../types";
import MockComputeProviderModule from "../../ignition/modules/mockComputeProvider";

const { ignition } = await network.connect();

export async function deployComputeProviderFixture() {
  const { mockComputeProvider } = await ignition.deploy(MockComputeProviderModule);

  return MockComputeProvider__factory.connect(await mockComputeProvider.getAddress());
}
