// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

const { ethers } = await network.connect();

import { PoseidonT3__factory } from "../../types/ethers-contracts";

export async function PoseidonT3Fixture(name?: string) {
  const [signer] = await ethers.getSigners();
  if (!signer) throw new Error("Bad getSigners output");
  const deployment = await (
    await ethers.getContractFactory(name || "PoseidonT3")
  ).deploy();

  return PoseidonT3__factory.connect(
    await deployment.getAddress(),
    signer.provider,
  );
}
