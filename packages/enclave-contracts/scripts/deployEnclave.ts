// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import hre from "hardhat";

import { autoCleanForLocalhost } from "./cleanIgnitionState";
import { deployAndSaveBondingRegistry } from "./deployAndSave/bondingRegistry";
import { deployAndSaveCiphernodeRegistryOwnable } from "./deployAndSave/ciphernodeRegistryOwnable";
import { deployAndSaveCommitteeSortition } from "./deployAndSave/committeeSortition";
import { deployAndSaveEnclave } from "./deployAndSave/enclave";
import { deployAndSaveEnclaveTicketToken } from "./deployAndSave/enclaveTicketToken";
import { deployAndSaveEnclaveToken } from "./deployAndSave/enclaveToken";
import { deployAndSaveMockStableToken } from "./deployAndSave/mockStableToken";
import { deployAndSavePoseidonT3 } from "./deployAndSave/poseidonT3";
import { deployAndSaveSlashingManager } from "./deployAndSave/slashingManager";
import { deployMocks } from "./deployMocks";

/**
 * Deploys the Enclave contracts
 */
export const deployEnclave = async (withMocks?: boolean) => {
  const { ethers } = await hre.network.connect();

  // Auto-clean state for local networks to prevent stale state issues
  const networkName = hre.globalOptions.network ?? "localhost";
  await autoCleanForLocalhost(networkName);

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

  const shouldDeployMocks = process.env.DEPLOY_MOCKS === "true" || withMocks;
  let feeTokenAddress: string;

  if (shouldDeployMocks) {
    console.log("Deploying mock Fee token...");
    const { mockStableToken } = await deployAndSaveMockStableToken({
      initialSupply: 1000000,
      hre,
    });
    feeTokenAddress = await mockStableToken.getAddress();
    console.log("MockFeeToken deployed to:", feeTokenAddress);
  } else {
    throw new Error(
      "Fee token address must be provided for production deployment",
    );
  }

  console.log("Deploying ENCL token...");
  const { enclaveToken } = await deployAndSaveEnclaveToken({
    owner: ownerAddress,
    hre,
  });
  const enclaveTokenAddress = await enclaveToken.getAddress();
  console.log("EnclaveToken deployed to:", enclaveTokenAddress);

  console.log("Deploying EnclaveTicketToken...");
  const { enclaveTicketToken } = await deployAndSaveEnclaveTicketToken({
    baseToken: feeTokenAddress,
    registry: addressOne,
    owner: ownerAddress,
    hre,
  });
  const enclaveTicketTokenAddress = await enclaveTicketToken.getAddress();
  console.log("EnclaveTicketToken deployed to:", enclaveTicketTokenAddress);

  console.log("Deploying SlashingManager...");
  const { slashingManager } = await deployAndSaveSlashingManager({
    admin: ownerAddress,
    bondingRegistry: addressOne,
    hre,
  });
  const slashingManagerAddress = await slashingManager.getAddress();
  console.log("SlashingManager deployed to:", slashingManagerAddress);

  console.log("Deploying BondingRegistry...");
  const { bondingRegistry } = await deployAndSaveBondingRegistry({
    owner: ownerAddress,
    ticketToken: enclaveTicketTokenAddress,
    licenseToken: enclaveTokenAddress,
    registry: addressOne,
    slashedFundsTreasury: ownerAddress,
    ticketPrice: ethers.parseUnits("10", 6).toString(),
    licenseRequiredBond: ethers.parseEther("100").toString(),
    minTicketBalance: 1,
    exitDelay: 7 * 24 * 60 * 60,
    hre,
  });
  const bondingRegistryAddress = await bondingRegistry.getAddress();
  console.log("BondingRegistry deployed to:", bondingRegistryAddress);

  console.log("Deploying CiphernodeRegistry...");
  const { ciphernodeRegistry } = await deployAndSaveCiphernodeRegistryOwnable({
    poseidonT3Address: poseidonT3,
    enclaveAddress: addressOne,
    owner: ownerAddress,
    hre,
  });
  const ciphernodeRegistryAddress = await ciphernodeRegistry.getAddress();
  console.log("CiphernodeRegistry deployed to:", ciphernodeRegistryAddress);

  console.log("Deploying CommitteeSortition...");
  const { committeeSortition } = await deployAndSaveCommitteeSortition({
    bondingRegistry: bondingRegistryAddress,
    ciphernodeRegistry: ciphernodeRegistryAddress,
    hre,
  });
  const committeeSortitionAddress = await committeeSortition.getAddress();
  console.log("CommitteeSortition deployed to:", committeeSortitionAddress);

  console.log("Deploying Enclave...");
  const { enclave } = await deployAndSaveEnclave({
    params: [encoded],
    owner: ownerAddress,
    maxDuration: THIRTY_DAYS_IN_SECONDS.toString(),
    registry: ciphernodeRegistryAddress,
    bondingRegistry: bondingRegistryAddress,
    feeToken: feeTokenAddress,
    poseidonT3Address: poseidonT3,
    hre,
  });
  const enclaveAddress = await enclave.getAddress();
  console.log("Enclave deployed to:", enclaveAddress);

  ///////////////////////////////////////////
  // Configure cross-contract dependencies
  ///////////////////////////////////////////

  console.log("Configuring cross-contract dependencies...");

  console.log("Setting Enclave address in CiphernodeRegistry...");
  await ciphernodeRegistry.setEnclave(enclaveAddress);

  console.log("Setting BondingRegistry address in CiphernodeRegistry...");
  await ciphernodeRegistry.setBondingRegistry(bondingRegistryAddress);

  console.log("Setting BondingRegistry address in EnclaveTicketToken...");
  await enclaveTicketToken.setRegistry(bondingRegistryAddress);

  console.log("Setting CiphernodeRegistry address in BondingRegistry...");
  await bondingRegistry.setRegistry(ciphernodeRegistryAddress);

  console.log("Setting BondingRegistry address in SlashingManager...");
  await slashingManager.setBondingRegistry(bondingRegistryAddress);

  console.log("Setting SlashingManager address in BondingRegistry...");
  await bondingRegistry.setSlashingManager(slashingManagerAddress);

  console.log("Setting Enclave as reward distributor in BondingRegistry...");
  await bondingRegistry.setRewardDistributor(enclaveAddress);

  console.log("Setting CommitteeSortition address in CiphernodeRegistry...");
  await ciphernodeRegistry.setCommitteeSortition(committeeSortitionAddress);

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

  console.log(`
    ============================================
    Deployment Complete!
    ============================================
    MockFeeToken: ${feeTokenAddress}
    EnclaveToken (ENCL): ${enclaveTokenAddress}
    EnclaveTicketToken: ${enclaveTicketTokenAddress}
    SlashingManager: ${slashingManagerAddress}
    BondingRegistry: ${bondingRegistryAddress}
    CommitteeSortition: ${committeeSortitionAddress}
    CiphernodeRegistry: ${ciphernodeRegistryAddress}
    Enclave: ${enclaveAddress}
    ============================================
  `);
};
