// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

import { NaiveRegistryFilter__factory } from "../../types";
import NaiveRegistryFilterModule from "../../ignition/modules/naiveRegistryFilter";

const { ethers, ignition } = await network.connect();

export async function naiveRegistryFilterFixture(
  owner: string,
  registry: string,
) {
  const [signer] = await ethers.getSigners();
  const { naiveRegistryFilter } = await ignition.deploy(NaiveRegistryFilterModule, {
    parameters: {
      NaiveRegistryFilter: {
        owner,
        ciphernodeRegistryAddress: registry,
      },
    },
  });

  return NaiveRegistryFilter__factory.connect(
    await naiveRegistryFilter.getAddress(),
    signer,
  );
}
