import { ethers } from "hardhat";

import { PoseidonT3__factory } from "../../types";

export async function PoseidonT3Fixture(name?: string) {
  const [signer] = await ethers.getSigners();
  const deployment = await (
    await ethers.getContractFactory(name || "PoseidonT3")
  ).deploy();

  return PoseidonT3__factory.connect(
    await deployment.getAddress(),
    signer.provider,
  );
}
