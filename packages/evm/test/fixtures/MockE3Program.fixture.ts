import { ethers } from "hardhat";

import { MockE3Program__factory } from "../../types/factories/contracts/test/MockE3Program__factory";

export async function deployE3ProgramFixture(inputValidatorAddress: string) {
  const deployment = await (
    await ethers.getContractFactory("MockE3Program")
  ).deploy(inputValidatorAddress);
  const deploymentAddress = await deployment.getAddress();
  return MockE3Program__factory.connect(deploymentAddress);
}
