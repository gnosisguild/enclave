import { ethers } from "hardhat";

import { MockInputValidatorPolicyFactory__factory } from "../../types/factories/contracts/test/MockInputValidatorPolicyFactory__factory";

export async function deployInputValidatorPolicyFactoryFixture() {
  const deployment = await (
    await ethers.getContractFactory("MockInputValidatorPolicyFactory")
  ).deploy();
  return MockInputValidatorPolicyFactory__factory.connect(
    await deployment.getAddress(),
  );
}
