// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { network } from "hardhat";

import { Enclave__factory } from "../../types";
import EnclaveModule from "../../ignition/modules/enclave";

const { ethers, ignition } = await network.connect();

export async function deployEnclaveFixture(
  owner: string,
  registry: string,
  maxDuration?: number,
) {
  const [signer] = await ethers.getSigners();
  const polynomial_degree = ethers.toBigInt(2048);
  const plaintext_modulus = ethers.toBigInt(1032193);
  const moduli = [ethers.toBigInt("18014398492704769")]; // 0x3FFFFFFF000001

  // Encode just the struct (NOT the function selector)
  const encoded = ethers.AbiCoder.defaultAbiCoder().encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );

  const { enclave } = await ignition.deploy(EnclaveModule, {
    parameters: {
      Enclave: {
        params: encoded,
        owner: owner,
        maxDuration: maxDuration || 60 * 60 * 24 * 30,
        registry: registry,
      },
    },
  });

  return Enclave__factory.connect(await enclave.getAddress(), signer);
}
