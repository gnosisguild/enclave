import { ethers } from "hardhat";

import { MockCyphernodeRegistry__factory } from "../../types";

export async function deployCyphernodeRegistryFixture(name?: string) {
  const [signer] = await ethers.getSigners();
  const deployment = await (await ethers.getContractFactory(name || "MockCyphernodeRegistry")).deploy();

  return MockCyphernodeRegistry__factory.connect(await deployment.getAddress(), signer.provider);
}
