import { ethers } from "hardhat";

import { MockOutputVerifier } from "../../types/contracts/test/MockOutputVerifier";
import { MockOutputVerifier__factory } from "../../types/factories/contracts/test/MockOutputVerifier__factory";

export async function deployMockOutputVerifierFixture() {
  const MockOutputVerifier = (await ethers.getContractFactory("MockOutputVerifier")) as MockOutputVerifier__factory;
  const mockOutputVerifier = (await MockOutputVerifier.deploy()) as MockOutputVerifier;
  const mockOutputVerifier_address = mockOutputVerifier.getAddress();
  return { mockOutputVerifier, mockOutputVerifier_address };
}
