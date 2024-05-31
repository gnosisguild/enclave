import { ethers } from "hardhat";

import { MockInputValidator } from "../../types/contracts/test/MockInputValidator";
import { MockInputValidator__factory } from "../../types/factories/contracts/test/MockInputValidator__factory";

export async function deployMockInputValidatorFixture() {
  const MockInputValidator = (await ethers.getContractFactory("MockInputValidator")) as MockInputValidator__factory;
  const mockInputValidator = (await MockInputValidator.deploy()) as MockInputValidator;
  const mockInputValidator_address = mockInputValidator.getAddress();
  return { mockInputValidator, mockInputValidator_address };
}
