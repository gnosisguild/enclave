import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";
import { ethers } from "hardhat";

import { Enclave__factory } from "../../types/factories/contracts/Enclave__factory";

export async function deployEnclaveFixture({
  owner,
  registry,
  maxDuration,
}: {
  owner?: SignerWithAddress;
  registry?: SignerWithAddress;
  maxDuration?: number;
} = {}) {
  // Contracts are deployed using the first signer/account by default
  const [account1, account2] = await ethers.getSigners();

  owner = owner || account1;
  registry = registry || account2;
  maxDuration = maxDuration || 60 * 60 * 24 * 30;

  const deployment = await (await ethers.getContractFactory("Enclave")).deploy(owner, registry, maxDuration);

  return Enclave__factory.connect(await deployment.getAddress(), owner);
}
