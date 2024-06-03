import { ethers } from "hardhat";

import { MockCypherNodeRegistry__factory } from "../../types/factories/contracts/test/MockCypherNodeRegistry__factory";

export async function deployCypherNodeRegistryFixture() {
  const deployment = await (await ethers.getContractFactory("MockCypherNodeRegistry")).deploy();

  return MockCypherNodeRegistry__factory.connect(await deployment.getAddress());
}
