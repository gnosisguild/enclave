import { ethers } from "hardhat";

import { MockE3Program__factory } from "../../types/factories/contracts/test/MockE3Program__factory";

export async function deployE3ProgramFixture(
  policyFactory: string,
  checker: string,
) {
  const deployment = await (
    await ethers.getContractFactory("MockE3Program")
  ).deploy(policyFactory, checker);
  return MockE3Program__factory.connect(await deployment.getAddress());
}
