import { ethers } from "hardhat";

import { MockComputationModule__factory } from "../../types/factories/contracts/test/MockComputationModule__factory";

export async function deployComputationModuleFixture() {
  const deployment = await (await ethers.getContractFactory("MockComputationModule")).deploy();

  return MockComputationModule__factory.connect(await deployment.getAddress());
}
