import { ethers } from "hardhat";

import { MockOutputVerifier__factory } from "../../types/factories/contracts/test/MockOutputVerifier__factory";

export async function deployOutputVerifierFixture() {
  const deployment = await (
    await ethers.getContractFactory("MockOutputVerifier")
  ).deploy();
  return MockOutputVerifier__factory.connect(await deployment.getAddress());
}
