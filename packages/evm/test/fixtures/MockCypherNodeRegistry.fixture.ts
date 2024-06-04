import { ethers } from "hardhat";

import { MockCyphernodeRegistry__factory } from "../../types/factories/contracts/test/MockCyphernodeRegistry__factory";

export async function deployCyphernodeRegistryFixture() {
  const deployment = await (await ethers.getContractFactory("MockCyphernodeRegistry")).deploy();

  return MockCyphernodeRegistry__factory.connect(await deployment.getAddress());
}
