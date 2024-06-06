import { ethers } from "hardhat";

import { MockExecutionModule__factory } from "../../types/factories/contracts/test/MockExecutionModule__factory";

export async function deployExecutionModuleFixture() {
  const deployment = await (
    await ethers.getContractFactory("MockExecutionModule")
  ).deploy();

  return MockExecutionModule__factory.connect(await deployment.getAddress());
}
