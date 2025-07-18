import { ethers } from "hardhat";

import { MockCiphernodeRegistry__factory } from "../../types";

export async function deployCiphernodeRegistryFixture(name?: string) {
  const [signer] = await ethers.getSigners();
  const deployment = await (
    await ethers.getContractFactory(name || "MockCiphernodeRegistry")
  ).deploy();

  return MockCiphernodeRegistry__factory.connect(
    await deployment.getAddress(),
    signer!.provider,
  );
}
