// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

import { CiphernodeRegistryOwnable__factory } from "../../types";
import CiphernodeRegistryModule from "../../ignition/modules/ciphernodeRegistry";

const { ethers, ignition } = await network.connect();

export async function deployCiphernodeRegistryOwnableFixture(
  owner: string,
  enclave: string,
) {
  const [signer] = await ethers.getSigners();
  const { cipherNodeRegistry } = await ignition.deploy(CiphernodeRegistryModule, {
    parameters: {
      CiphernodeRegistry: {
        enclaveAddress: enclave,
        owner: owner,
      },
    },
  });

  return CiphernodeRegistryOwnable__factory.connect(
    await cipherNodeRegistry.getAddress(),
    signer,
  );
}
