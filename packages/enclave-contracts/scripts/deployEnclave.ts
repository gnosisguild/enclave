// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import hre from "hardhat";

import { autoCleanForLocalhost } from "./cleanIgnitionState";
import { deployAndSaveBfvDecryptionVerifier } from "./deployAndSave/bfvDecryptionVerifier";
import { deployAndSaveBfvPkVerifier } from "./deployAndSave/bfvPkVerifier";
import { deployAndSaveBondingRegistry } from "./deployAndSave/bondingRegistry";
import { deployAndSaveCiphernodeRegistryOwnable } from "./deployAndSave/ciphernodeRegistryOwnable";
import { deployAndSaveE3RefundManager } from "./deployAndSave/e3RefundManager";
import { deployAndSaveEnclave } from "./deployAndSave/enclave";
import { deployAndSaveEnclaveTicketToken } from "./deployAndSave/enclaveTicketToken";
import { deployAndSaveEnclaveToken } from "./deployAndSave/enclaveToken";
import { deployAndSaveMockStableToken } from "./deployAndSave/mockStableToken";
import { deployAndSavePoseidonT3 } from "./deployAndSave/poseidonT3";
import { deployAndSaveSlashingManager } from "./deployAndSave/slashingManager";
import { deployAndSaveAllVerifiers } from "./deployAndSave/verifiers";
import { deployMocks } from "./deployMocks";

/**
 * Default timeout configuration (in seconds)
 */
const DEFAULT_TIMEOUT_CONFIG = {
  committeeFormationWindow: 3600,
  dkgWindow: 7200,
  computeWindow: 86400,
  decryptionWindow: 3600,
};

/** Circuit names required for BFV ZK verification in this script */
const THRESHOLD_DECRYPTED_SHARES_AGGREGATION_VERIFIER =
  "ThresholdDecryptedSharesAggregationVerifier";
const THRESHOLD_PK_AGGREGATION_VERIFIER = "ThresholdPkAggregationVerifier";

/**
 * Deploys the Enclave contracts
 */
export const deployEnclave = async (
  withMocks?: boolean,
  withZKVerification?: boolean,
) => {
  const { ethers } = await hre.network.connect();

  // Auto-clean state for local networks to prevent stale state issues
  const networkName = hre.globalOptions.network ?? "localhost";
  await autoCleanForLocalhost(networkName);

  const [owner] = await ethers.getSigners();

  const ownerAddress = await owner.getAddress();

  const polynomial_degree = ethers.toBigInt(512);
  const plaintext_modulus = ethers.toBigInt(100);
  const moduli = [
    ethers.toBigInt("0xffffee001"),
    ethers.toBigInt("0xffffc4001"),
  ];
  const error1_variance = "3";
  const encoded = ethers.AbiCoder.defaultAbiCoder().encode(
    [
      "tuple(uint256 degree, uint256 plaintext_modulus, uint256[] moduli, string error1_variance)",
    ],
    [[polynomial_degree, plaintext_modulus, moduli, error1_variance]],
  );

  const THIRTY_DAYS_IN_SECONDS = 60 * 60 * 24 * 30;
  const SORTITION_SUBMISSION_WINDOW = 10;
  const addressOne = "0x0000000000000000000000000000000000000001";

  const poseidonT3 = await deployAndSavePoseidonT3({ hre });

  const shouldDeployMocks = process.env.DEPLOY_MOCKS === "true" || withMocks;
  const shouldHaveZKVerification =
    process.env.ENABLE_ZK_VERIFICATION === "true" || withZKVerification;

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
    hre,
  });
  const slashingManagerAddress = await slashingManager.getAddress();
  console.log("SlashingManager deployed to:", slashingManagerAddress);

  console.log("Deploying CiphernodeRegistry...");
  const { ciphernodeRegistry } = await deployAndSaveCiphernodeRegistryOwnable({
    poseidonT3Address: poseidonT3,
    owner: ownerAddress,
    submissionWindow: SORTITION_SUBMISSION_WINDOW,
    hre,
  });
  const ciphernodeRegistryAddress = await ciphernodeRegistry.getAddress();
  console.log("CiphernodeRegistry deployed to:", ciphernodeRegistryAddress);

  console.log("Deploying BondingRegistry...");
  const { bondingRegistry } = await deployAndSaveBondingRegistry({
    owner: ownerAddress,
    ticketToken: enclaveTicketTokenAddress,
    licenseToken: enclaveTokenAddress,
    registry: ciphernodeRegistryAddress,
    slashedFundsTreasury: ownerAddress,
    ticketPrice: ethers.parseUnits("10", 6).toString(),
    licenseRequiredBond: ethers.parseEther("100").toString(),
    minTicketBalance: 1,
    exitDelay: 7 * 24 * 60 * 60,
    hre,
  });
  const bondingRegistryAddress = await bondingRegistry.getAddress();
  console.log("BondingRegistry deployed to:", bondingRegistryAddress);

  console.log("Deploying Enclave...");
  const { enclave } = await deployAndSaveEnclave({
    params: [encoded],
    owner: ownerAddress,
    maxDuration: THIRTY_DAYS_IN_SECONDS.toString(),
    registry: ciphernodeRegistryAddress,
    bondingRegistry: bondingRegistryAddress,
    e3RefundManager: addressOne, // placeholder, will be updated
    feeToken: feeTokenAddress,
    timeoutConfig: DEFAULT_TIMEOUT_CONFIG,
    hre,
  });
  const enclaveAddress = await enclave.getAddress();
  console.log("Enclave deployed to:", enclaveAddress);

  console.log("Deploying E3RefundManager...");
  const { e3RefundManager } = await deployAndSaveE3RefundManager({
    owner: ownerAddress,
    enclave: enclaveAddress,
    treasury: ownerAddress, // Protocol treasury
    hre,
  });
  const e3RefundManagerAddress = await e3RefundManager.getAddress();
  console.log("E3RefundManager deployed to:", e3RefundManagerAddress);

  console.log("Setting E3RefundManager in Enclave...");
  await enclave.setE3RefundManager(e3RefundManagerAddress);

  ///////////////////////////////////////////
  // Configure cross-contract dependencies
  ///////////////////////////////////////////

  console.log("Configuring cross-contract dependencies...");

  console.log("Setting Enclave address in CiphernodeRegistry...");
  await ciphernodeRegistry.setEnclave(enclaveAddress);

  console.log("Setting BondingRegistry address in CiphernodeRegistry...");
  await ciphernodeRegistry.setBondingRegistry(bondingRegistryAddress);

  console.log("Setting Submission Window in CiphernodeRegistry...");
  console.log("SORTITION_SUBMISSION_WINDOW:", SORTITION_SUBMISSION_WINDOW);
  await ciphernodeRegistry.setSortitionSubmissionWindow(
    SORTITION_SUBMISSION_WINDOW,
  );

  console.log("Setting BondingRegistry address in EnclaveTicketToken...");
  await enclaveTicketToken.setRegistry(bondingRegistryAddress);

  console.log("Setting CiphernodeRegistry address in BondingRegistry...");
  await bondingRegistry.setRegistry(ciphernodeRegistryAddress);

  console.log("Setting Enclave address in SlashingManager...");
  await slashingManager.setEnclave(enclaveAddress);

  console.log("Setting BondingRegistry address in SlashingManager...");
  await slashingManager.setBondingRegistry(bondingRegistryAddress);

  console.log("Setting CiphernodeRegistry address in SlashingManager...");
  await slashingManager.setCiphernodeRegistry(ciphernodeRegistryAddress);

  console.log("Setting E3RefundManager address in SlashingManager...");
  await slashingManager.setE3RefundManager(e3RefundManagerAddress);

  console.log("Setting SlashingManager address in Enclave...");
  await enclave.setSlashingManager(slashingManagerAddress);

  console.log("Setting SlashingManager address in BondingRegistry...");
  await bondingRegistry.setSlashingManager(slashingManagerAddress);

  console.log("Setting SlashingManager address in CiphernodeRegistry...");
  await ciphernodeRegistry.setSlashingManager(slashingManagerAddress);

  console.log("Setting Enclave as reward distributor in BondingRegistry...");
  await bondingRegistry.setRewardDistributor(enclaveAddress);

  // E3RefundManager already has correct enclave from deployment

  // Initialize committee size thresholds [threshold, total]
  console.log("Setting committee thresholds...");
  // Micro: threshold=1, total=3
  await enclave.setCommitteeThresholds(0, [1, 3]);
  // Small: threshold=2, total=5
  await enclave.setCommitteeThresholds(1, [2, 5]);
  // Medium and Large can be set later as needed
  console.log("Committee thresholds set (Micro=[1,3], Small=[2,5])");

  const encryptionSchemeId = ethers.keccak256(ethers.toUtf8Bytes("fhe.rs:BFV"));

  // Set pricing config with protocol treasury
  console.log("Setting pricing config...");
  await enclave.setPricingConfig({
    keyGenFixedPerNode: 50000, // 0.05 USDC
    keyGenPerEncryptionProof: 25000, // 0.025 USDC
    coordinationPerPair: 5000, // 0.005 USDC
    availabilityPerNodePerSec: 20, // 0.00002 USDC
    decryptionPerNode: 150000, // 0.15 USDC
    publicationBase: 500000, // 0.50 USDC
    verificationPerProof: 2000, // 0.002 USDC
    protocolTreasury: ownerAddress,
    marginBps: 1000, // 10%
    protocolShareBps: 2000, // 20%
    minCommitteeSize: 0,
    minThreshold: 0,
  });
  console.log("Pricing config set (treasury:", ownerAddress, ")");

  if (shouldDeployMocks) {
    const {
      decryptionVerifierAddress: mockDecryptionVerifierAddress,
      pkVerifierAddress: mockPkVerifierAddress,
      e3ProgramAddress,
    } = await deployMocks();

    console.log("encryptionSchemeId", encryptionSchemeId);

    if (!shouldHaveZKVerification && mockDecryptionVerifierAddress) {
      const deployedDecryptionVerifier =
        await enclave.decryptionVerifiers(encryptionSchemeId);
      if (deployedDecryptionVerifier === mockDecryptionVerifierAddress) {
        console.log(`DecryptionVerifier already set in Enclave contract`);
      } else {
        const tx = await enclave.setDecryptionVerifier(
          encryptionSchemeId,
          mockDecryptionVerifierAddress,
        );
        await tx.wait();
        console.log(
          `Successfully set MockDecryptionVerifier in Enclave contract`,
        );
      }
    }

    if (!shouldHaveZKVerification && mockPkVerifierAddress) {
      const deployedPkVerifier = await enclave.pkVerifiers(encryptionSchemeId);
      if (deployedPkVerifier === mockPkVerifierAddress) {
        console.log(`PkVerifier already set in Enclave contract`);
      } else {
        const tx = await enclave.setPkVerifier(
          encryptionSchemeId,
          mockPkVerifierAddress,
        );
        await tx.wait();
        console.log(`Successfully set MockPkVerifier in Enclave contract`);
      }
    }

    const tx = await enclave.enableE3Program(e3ProgramAddress);
    await tx.wait();
    console.log(`Successfully enabled E3 Program in Enclave contract`);
  }

  let verifierDeployments: Record<string, string> = {};
  if (shouldHaveZKVerification) {
    console.log("Deploying circuit verifiers...");
    verifierDeployments = await deployAndSaveAllVerifiers(hre);
    const requiredVerifierNames = [
      THRESHOLD_DECRYPTED_SHARES_AGGREGATION_VERIFIER,
      THRESHOLD_PK_AGGREGATION_VERIFIER,
    ] as const;
    for (const name of requiredVerifierNames) {
      const addr = verifierDeployments[name];
      if (!addr?.trim()) {
        throw new Error(
          `ZK verification enabled but "${name}" is missing from verifier deployments ` +
            `(got ${verifierDeployments[name] === undefined ? "undefined" : JSON.stringify(addr)}). ` +
            `Ensure deployAndSaveAllVerifiers discovers and deploys this circuit, or fix verifier artifacts.`,
        );
      }
    }
  } else {
    console.log("Skipping circuit verifiers (ENABLE_ZK_VERIFICATION not set)");
  }
  const verifierEntries = Object.entries(verifierDeployments);

  if (shouldHaveZKVerification) {
    console.log("Deploying BfvDecryptionVerifier and registering for prod...");
    const { bfvDecryptionVerifier } =
      await deployAndSaveBfvDecryptionVerifier(hre);
    const bfvDecryptionVerifierAddress =
      await bfvDecryptionVerifier.getAddress();
    const deployedDecryptionVerifier =
      await enclave.decryptionVerifiers(encryptionSchemeId);
    if (deployedDecryptionVerifier !== bfvDecryptionVerifierAddress) {
      const tx = await enclave.setDecryptionVerifier(
        encryptionSchemeId,
        bfvDecryptionVerifierAddress,
      );
      await tx.wait();
      console.log("Successfully set BfvDecryptionVerifier in Enclave contract");
    }
  }

  if (shouldHaveZKVerification) {
    console.log("Deploying BfvPkVerifier and registering for prod...");
    const { bfvPkVerifier } = await deployAndSaveBfvPkVerifier(hre);
    const bfvPkVerifierAddress = await bfvPkVerifier.getAddress();
    const deployedPkVerifier = await enclave.pkVerifiers(encryptionSchemeId);
    if (deployedPkVerifier !== bfvPkVerifierAddress) {
      const tx = await enclave.setPkVerifier(
        encryptionSchemeId,
        bfvPkVerifierAddress,
      );
      await tx.wait();
      console.log("Successfully set BfvPkVerifier in Enclave contract");
    }
  }

  const verifierLines =
    verifierEntries.length > 0
      ? verifierEntries.map(([name, addr]) => `    ${name}: ${addr}`).join("\n")
      : "    (none)";

  const decryptionVerifierAddress =
    await enclave.decryptionVerifiers(encryptionSchemeId);
  const pkVerifierAddress = await enclave.pkVerifiers(encryptionSchemeId);

  console.log(`
    ============================================
    Deployment Complete!
    ============================================
    MockFeeToken: ${feeTokenAddress}
    EnclaveToken (ENCL): ${enclaveTokenAddress}
    EnclaveTicketToken: ${enclaveTicketTokenAddress}
    SlashingManager: ${slashingManagerAddress}
    BondingRegistry: ${bondingRegistryAddress}
    CiphernodeRegistry: ${ciphernodeRegistryAddress}
    E3RefundManager: ${e3RefundManagerAddress}
    Enclave: ${enclaveAddress}
    DecryptionVerifier (BFV): ${decryptionVerifierAddress}
    PkVerifier (BFV): ${pkVerifierAddress}
    Circuit Verifiers:
${verifierLines}
    ============================================
  `);
};
