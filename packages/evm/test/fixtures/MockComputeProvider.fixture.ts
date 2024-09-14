import { ethers } from "hardhat";

import { MockComputeProvider__factory } from "../../types/factories/contracts/test/MockComputeProvider__factory";

export async function deployComputeProviderFixture() {
  const deployment = await (
    await ethers.getContractFactory("MockComputeProvider")
  ).deploy();

  return MockComputeProvider__factory.connect(await deployment.getAddress());
}
