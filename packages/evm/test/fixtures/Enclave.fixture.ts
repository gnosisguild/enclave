import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";
import { ethers } from "hardhat";

import { Enclave__factory } from "../../types/factories/contracts/Enclave__factory";

export async function deployEnclaveFixture({
  owner,
  registry,
  maxDuration = 60 * 60 * 24 * 30,
}: {
  owner: SignerWithAddress;
  registry: string;
  maxDuration?: number;
}) {
  const deployment = await (
    await ethers.getContractFactory("Enclave")
  ).deploy(owner, registry, maxDuration);

  return Enclave__factory.connect(await deployment.getAddress(), owner);
}
