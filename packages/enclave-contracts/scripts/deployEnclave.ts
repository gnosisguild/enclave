// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import hre from "hardhat";

import { deployAndSaveCiphernodeRegistryOwnable } from "./deployAndSave/ciphernodeRegistryOwnable";
import { deployAndSaveEnclave } from "./deployAndSave/enclave";
import { deployAndSaveNaiveRegistryFilter } from "./deployAndSave/naiveRegistryFilter";
import { deployAndSavePoseidonT3 } from "./deployAndSave/poseidonT3";
import { deployMocks } from "./deployMocks";
import { deployAndSavePoseidonT3 } from "./deployAndSave/poseidonT3";

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

  const poseidonT3 = await deployAndSavePoseidonT3({ hre });

  console.log("Deploying Enclave");
  const { enclave } = await deployAndSaveEnclave({
    params: [encoded],
    owner: ownerAddress,
    maxDuration: THIRTY_DAYS_IN_SECONDS.toString(),
    registry: addressOne,
    poseidonT3Address: poseidonT3,
    hre,
  });

  const enclaveAddress = await enclave.getAddress();

  console.log("Deploying CiphernodeRegistry");
  const { ciphernodeRegistry } = await deployAndSaveCiphernodeRegistryOwnable({
    enclaveAddress: enclaveAddress,
    owner: ownerAddress,
    poseidonT3Address: poseidonT3,
    hre,
  });

  const ciphernodeRegistryAddress = await ciphernodeRegistry.getAddress();

  console.log("Deploying NaiveRegistryFilter");
  const { naiveRegistryFilter } = await deployAndSaveNaiveRegistryFilter({
    ciphernodeRegistryAddress: ciphernodeRegistryAddress,
    owner: ownerAddress,
    hre,
  });

  const naiveRegistryFilterAddress = await naiveRegistryFilter.getAddress();

  const registryAddress = await enclave.ciphernodeRegistry();

  console.log("Setting CiphernodeRegistry in Enclave");
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
    console.log("Deploying Mocks");
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
