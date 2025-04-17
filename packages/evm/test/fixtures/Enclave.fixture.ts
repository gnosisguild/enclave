import { ethers } from "hardhat";

import { Enclave__factory } from "../../types/factories/contracts/Enclave__factory";

export async function deployEnclaveFixture(
  owner: string,
  registry: string,
  poseidonT3: string,
  maxDuration?: number,
) {
  const [signer] = await ethers.getSigners();
  const polynomial_degree = ethers.toBigInt(2048);
  const plaintext_modulus = ethers.toBigInt(1032193);
  const moduli = [ethers.toBigInt("4503599626321921")];

  // Encode just the struct (NOT the function selector)
  const encoded = ethers.AbiCoder.defaultAbiCoder().encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );
  const deployment = await (
    await ethers.getContractFactory("Enclave", {
      libraries: {
        PoseidonT3: poseidonT3,
      },
    })
  ).deploy(owner, registry, maxDuration || 60 * 60 * 24 * 30, [encoded]);

  return Enclave__factory.connect(await deployment.getAddress(), signer);
}
