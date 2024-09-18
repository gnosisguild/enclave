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
    signer.provider,
  );
}
