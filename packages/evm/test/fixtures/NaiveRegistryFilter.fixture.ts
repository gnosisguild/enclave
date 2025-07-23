// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { ethers } from "hardhat";

import { NaiveRegistryFilter__factory } from "../../types";

export async function naiveRegistryFilterFixture(
  owner: string,
  registry: string,
  name?: string,
) {
  const [signer] = await ethers.getSigners();
  const deployment = await (
    await ethers.getContractFactory(name || "NaiveRegistryFilter")
  ).deploy(owner, registry);

  return NaiveRegistryFilter__factory.connect(
    await deployment.getAddress(),
    signer,
  );
}
