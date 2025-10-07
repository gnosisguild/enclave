// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import hre from "hardhat";

import { deployAndSaveCiphernodeRegistryOwnable } from "./deployAndSave/ciphernodeRegistryOwnable";
import { deployAndSaveEnclave } from "./deployAndSave/enclave";
import { deployAndSaveNaiveRegistryFilter } from "./deployAndSave/naiveRegistryFilter";
import { deployMocks } from "./deployMocks";

/**
 * Deploys the Enclave contracts
 */
export const deployEnclave = async (withMocks?: boolean) => {
  const { ethers } = await hre.network.connect();

  const [owner] = await ethers.getSigners();

  const ownerAddress = await owner.getAddress();

  const polynomial_degree = ethers.toBigInt(2048);
  const plaintext_modulus = ethers.toBigInt(1032193);
  const moduli = [ethers.toBigInt("18014398492704769")];

  const encoded = ethers.AbiCoder.defaultAbiCoder().encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );

  const THIRTY_DAYS_IN_SECONDS = 60 * 60 * 24 * 30;
  const addressOne = "0x0000000000000000000000000000000000000001";

  const { enclave } = await deployAndSaveEnclave({
    params: encoded,
    owner: ownerAddress,
    maxDuration: THIRTY_DAYS_IN_SECONDS.toString(),
    registry: addressOne,
    hre,
  });

  const enclaveAddress = await enclave.getAddress();

  const { ciphernodeRegistry } = await deployAndSaveCiphernodeRegistryOwnable({
    enclaveAddress: enclaveAddress,
    owner: ownerAddress,
    hre,
  });

  const ciphernodeRegistryAddress = await ciphernodeRegistry.getAddress();

  const { naiveRegistryFilter } = await deployAndSaveNaiveRegistryFilter({
    ciphernodeRegistryAddress: ciphernodeRegistryAddress,
    owner: ownerAddress,
    hre,
  });

  const naiveRegistryFilterAddress = await naiveRegistryFilter.getAddress();

  const registryAddress = await enclave.ciphernodeRegistry();

  if (registryAddress === ciphernodeRegistryAddress) {
    console.log(`Enclave contract already has registry`);
  } else {
    const tx = await enclave.setCiphernodeRegistry(ciphernodeRegistryAddress);
    await tx.wait();

    console.log(`Enclave contract updated with registry`);
  }

  console.log(`
        Deployments:
        ----------------------------------------------------------------------
        Enclave: ${enclaveAddress}
        CiphernodeRegistry: ${ciphernodeRegistryAddress}
        NaiveRegistryFilter: ${naiveRegistryFilterAddress}
        `);

  // Deploy mocks only if specified
  const shouldDeployMocks = process.env.DEPLOY_MOCKS === "true" || withMocks;

  if (shouldDeployMocks) {
    const { decryptionVerifierAddress, e3ProgramAddress } = await deployMocks();

    const encryptionSchemeId = ethers.keccak256(
      ethers.toUtf8Bytes("fhe.rs:BFV"),
    );

    console.log("encryptionSchemeId", encryptionSchemeId);

    const deployedDecryptionVerifier =
      await enclave.decryptionVerifiers(encryptionSchemeId);
    if (deployedDecryptionVerifier === decryptionVerifierAddress) {
      console.log(`DecryptionVerifier already set in Enclave contract`);
    } else {
      const tx = await enclave.setDecryptionVerifier(
        encryptionSchemeId,
        decryptionVerifierAddress,
      );
      await tx.wait();
      console.log(
        `Successfully set MockDecryptionVerifier in Enclave contract`,
      );
    }

    const tx = await enclave.enableE3Program(e3ProgramAddress);
    await tx.wait();
    console.log(`Successfully enabled E3 Program in Enclave contract`);
  }
};
