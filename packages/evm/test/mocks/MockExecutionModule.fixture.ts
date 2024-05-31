import { ethers } from "hardhat";

import { MockExecutionModule } from "../../types/contracts/test/MockExecutionModule";
import { MockExecutionModule__factory } from "../../types/factories/contracts/test/MockExecutionModule__factory";

export async function deployMockExecutionModuleFixture() {
  const MockExecutionModule = (await ethers.getContractFactory("MockExecutionModule")) as MockExecutionModule__factory;
  const mockExecutionModule = (await MockExecutionModule.deploy()) as MockExecutionModule;
  const mockExecutionModule_address = await mockExecutionModule.getAddress();

  return { mockExecutionModule, mockExecutionModule_address };
}
