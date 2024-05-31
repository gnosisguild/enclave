import { ethers } from "hardhat";

import type { Enclave } from "../../types/contracts/Enclave";
import type { Enclave__factory } from "../../types/factories/contracts/Enclave__factory";

export async function deployEnclaveFixture() {
  // Contracts are deployed using the first signer/account by default
  const [owner, otherAccount] = await ethers.getSigners();
  const maxDuration = 60 * 60 * 24 * 30;

  const Enclave = (await ethers.getContractFactory("Enclave")) as Enclave__factory;
  const enclave = (await Enclave.deploy(owner.address, maxDuration)) as Enclave;
  const enclave_address = await enclave.getAddress();

  return { Enclave, enclave, enclave_address, maxDuration, owner, otherAccount };
}
