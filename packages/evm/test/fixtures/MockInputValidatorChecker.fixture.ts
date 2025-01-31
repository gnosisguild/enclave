import { ethers } from "hardhat";

import { MockInputValidatorChecker__factory } from "../../types/factories/contracts/test/MockInputValidatorChecker__factory";

export async function deployInputValidatorCheckerFixture(validator: string) {
  const deployment = await (
    await ethers.getContractFactory("MockInputValidatorChecker")
  ).deploy([validator]);
  return MockInputValidatorChecker__factory.connect(
    await deployment.getAddress(),
  );
}
