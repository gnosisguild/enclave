import { ethers } from "hardhat";

import { MockInputValidator__factory } from "../../types/factories/contracts/test/MockInputValidator__factory";

export async function deployInputValidatorFixture() {
  const deployment = await (await ethers.getContractFactory("MockInputValidator")).deploy();
  return MockInputValidator__factory.connect(await deployment.getAddress());
}
