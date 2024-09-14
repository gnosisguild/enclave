import { ethers } from "hardhat";

import { MockDecryptionVerifier__factory } from "../../types/factories/contracts/test/MockDecryptionVerifier__factory";

export async function deployDecryptionVerifierFixture() {
  const deployment = await (
    await ethers.getContractFactory("MockDecryptionVerifier")
  ).deploy();
  return MockDecryptionVerifier__factory.connect(await deployment.getAddress());
}
