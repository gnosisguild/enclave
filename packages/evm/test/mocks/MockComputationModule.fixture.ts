import { ethers } from "hardhat";

import type { MockComputationModule } from "../../types/contracts/test/MockComputationModule";
import type { MockComputationModule__factory } from "../../types/factories/contracts/test/MockComputationModule__factory";

export async function deployMockComputationModuleFixture() {
  // Contracts are deployed using the first signer/account by default
  const [owner, otherAccount] = await ethers.getSigners();

  const MockComputationModule = (await ethers.getContractFactory(
    "MockComputationModule",
  )) as MockComputationModule__factory;
  const mockComputationModule = (await MockComputationModule.deploy()) as MockComputationModule;
  const mockComputationModule_address = await mockComputationModule.getAddress();

  return { mockComputationModule, mockComputationModule_address, owner, otherAccount };
}
