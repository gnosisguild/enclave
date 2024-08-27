import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";
import { ethers } from "hardhat";
import { PoseidonT3, proxy } from "poseidon-solidity";

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
  if ((await ethers.provider.getCode(proxy.address)) === "0x") {
    // fund the keyless account
    await owner.sendTransaction({
      to: proxy.from,
      value: proxy.gas,
    });

    // then send the presigned transaction deploying the proxy
    await ethers.provider.broadcastTransaction(proxy.tx);
  }

  // Then deploy the hasher, if needed
  if ((await ethers.provider.getCode(PoseidonT3.address)) === "0x") {
    await owner.sendTransaction({
      to: proxy.address,
      data: PoseidonT3.data,
    });
  }

  const poseidonDeployment = await await ethers.getContractAt(
    "PoseidonT3",
    proxy.address,
  );

  const imtDeployment = await (
    await ethers.getContractFactory("BinaryIMT", {
      libraries: {
        PoseidonT3: await poseidonDeployment.getAddress(),
      },
    })
  ).deploy();

  const deployment = await (
    await ethers.getContractFactory("Enclave", {
      libraries: {
        BinaryIMT: await imtDeployment.getAddress(),
      },
    })
  ).deploy(owner, registry, maxDuration);

  return Enclave__factory.connect(await deployment.getAddress(), owner);
}
