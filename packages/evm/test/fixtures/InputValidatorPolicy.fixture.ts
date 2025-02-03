import { ethers } from "hardhat";

import { InputValidatorPolicy__factory } from "../../types/factories/contracts/excubiae/InputValidatorPolicy__factory";

export async function deployInputValidatorPolicyFixture(checker: string) {
  const deployment = await (
    await ethers.getContractFactory("InputValidatorPolicy")
  ).deploy(checker);
  return InputValidatorPolicy__factory.connect(await deployment.getAddress());
}
