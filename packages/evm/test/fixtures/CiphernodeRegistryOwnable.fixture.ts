import { ethers } from "hardhat";

import { CiphernodeRegistryOwnable__factory } from "../../types";

export async function deployCiphernodeRegistryOwnableFixture(
  owner: string,
  enclave: string,
  poseidonT3: string,
  name?: string,
) {
  const [signer] = await ethers.getSigners();
  const deployment = await (
    await ethers.getContractFactory(name || "CiphernodeRegistryOwnable", {
      libraries: {
        PoseidonT3: poseidonT3,
      },
    })
  ).deploy(owner, enclave);

  return CiphernodeRegistryOwnable__factory.connect(
    await deployment.getAddress(),
    signer,
  );
}
