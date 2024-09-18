import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";
import { ethers } from "hardhat";

import { Enclave__factory } from "../../types/factories/contracts/Enclave__factory";

export async function deployEnclaveFixture(
  owner: string,
  registry: string,
  poseidonT3: string,
  maxDuration?: number,
) {
  const [signer] = await ethers.getSigners();
  const deployment = await (
    await ethers.getContractFactory("Enclave", {
      libraries: {
        PoseidonT3: poseidonT3,
      },
    })
  ).deploy(owner, registry, maxDuration || 60 * 60 * 24 * 30);

  return Enclave__factory.connect(await deployment.getAddress(), signer);
}
