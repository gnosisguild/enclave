import { ethers } from "hardhat";

import { MockInputValidatorFactory__factory } from "../../types/factories/contracts/test/MockInputValidatorFactory__factory";

export async function deployInputValidatorFactoryFixture() {
  const deployment = await (
    await ethers.getContractFactory("MockInputValidatorFactory")
  ).deploy();
  return MockInputValidatorFactory__factory.connect(
    await deployment.getAddress(),
  );
}
