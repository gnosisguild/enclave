import { ethers } from "hardhat";

import { MockE3Program__factory } from "../../types/factories/contracts/test/MockE3Program__factory";

export async function deployE3ProgramFixture(
  policyFactory: string,
  inputValidatorFactory: string,
  checker: string,
  inputLimit: number,
) {
  const deployment = await (
    await ethers.getContractFactory("MockE3Program")
  ).deploy(policyFactory, inputValidatorFactory, checker, inputLimit);
  return MockE3Program__factory.connect(await deployment.getAddress());
}
