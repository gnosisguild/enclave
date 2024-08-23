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
  const poseidonDeployment = await (
    await ethers.getContractFactory("PoseidonT3")
  ).deploy();

  const imtDeployment = await (
    await ethers.getContractFactory("BinaryIMT", {
      libraries: {
        PoseidonT3: poseidonDeployment.getAddress(),
      },
    })
  ).deploy();

  const deployment = await (
    await ethers.getContractFactory("Enclave", {
      Libraries: {
        BinaryIMT: imtDeployment.getAddress(),
      },
    })
  ).deploy(owner, registry, maxDuration);

  return Enclave__factory.connect(await deployment.getAddress(), owner);
}
