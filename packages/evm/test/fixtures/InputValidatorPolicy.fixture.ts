import { ethers } from "hardhat";

import { InputValidatorPolicy__factory } from "../../types/factories/contracts/excubiae/InputValidatorPolicy__factory";

export async function deployInputValidatorPolicyFixture(
  checker: string,
  inputLimit: number,
) {
  console.log("Deploying with inputLimit: " + inputLimit);
  const deployment = await (
    await ethers.getContractFactory("InputValidatorPolicy")
  ).deploy(checker, inputLimit);
  return InputValidatorPolicy__factory.connect(await deployment.getAddress());
}
