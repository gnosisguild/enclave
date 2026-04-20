// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import type { Signer } from "ethers";
import { network } from "hardhat";

import BondingRegistryModule from "../../ignition/modules/bondingRegistry";
import CiphernodeRegistryModule from "../../ignition/modules/ciphernodeRegistry";
import E3RefundManagerModule from "../../ignition/modules/e3RefundManager";
import EnclaveModule from "../../ignition/modules/enclave";
import EnclaveTicketTokenModule from "../../ignition/modules/enclaveTicketToken";
import EnclaveTokenModule from "../../ignition/modules/enclaveToken";
import mockComputeProviderModule from "../../ignition/modules/mockComputeProvider";
import MockDecryptionVerifierModule from "../../ignition/modules/mockDecryptionVerifier";
import MockE3ProgramModule from "../../ignition/modules/mockE3Program";
import MockPkVerifierModule from "../../ignition/modules/mockPkVerifier";
import MockStableTokenModule from "../../ignition/modules/mockStableToken";
import SlashingManagerModule from "../../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryOwnableFactory,
  Enclave__factory as EnclaveFactory,
  MockUSDC__factory as MockUSDCFactory,
} from "../../types";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture, time, mine } = networkHelpers;

describe("E3 Pricing", function () {
  const THIRTY_DAYS_IN_SECONDS = 60 * 60 * 24 * 30;
  const SORTITION_SUBMISSION_WINDOW = 10;
  const addressOne = "0x0000000000000000000000000000000000000001";

  const timeoutConfig = {
    committeeFormationWindow: 3600,
    dkgWindow: 3600,
    computeWindow: 3600,
    decryptionWindow: 3600,
  };

  // Default pricing config matching initialize() defaults
  const defaultPricingConfig = {
    keyGenFixedPerNode: 50000n,
    keyGenPerEncryptionProof: 25000n,
    coordinationPerPair: 5000n,
    availabilityPerNodePerSec: 20n,
    decryptionPerNode: 150000n,
    publicationBase: 500000n,
    verificationPerProof: 2000n,
    protocolTreasury: ethers.ZeroAddress,
    marginBps: 1000,
    protocolShareBps: 0,
    dkgUtilizationBps: 2500,
    computeUtilizationBps: 5000,
    decryptUtilizationBps: 2500,
    minCommitteeSize: 0,
    minThreshold: 0,
  };

  // Convert ethers Result to a plain object that can be spread
  const toPlainConfig = (pc: any) => ({
    keyGenFixedPerNode: pc.keyGenFixedPerNode,
    keyGenPerEncryptionProof: pc.keyGenPerEncryptionProof,
    coordinationPerPair: pc.coordinationPerPair,
    availabilityPerNodePerSec: pc.availabilityPerNodePerSec,
    decryptionPerNode: pc.decryptionPerNode,
    publicationBase: pc.publicationBase,
    verificationPerProof: pc.verificationPerProof,
    protocolTreasury: pc.protocolTreasury,
    marginBps: pc.marginBps,
    protocolShareBps: pc.protocolShareBps,
    dkgUtilizationBps: pc.dkgUtilizationBps,
    computeUtilizationBps: pc.computeUtilizationBps,
    decryptUtilizationBps: pc.decryptUtilizationBps,
    minCommitteeSize: pc.minCommitteeSize,
    minThreshold: pc.minThreshold,
  });

  const inputWindowDuration = 300;

  const encryptionSchemeId =
    "0x2c2a814a0495f913a3a312fc4771e37552bc14f8a2d4075a08122d356f0849c6";

  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  const polynomial_degree = ethers.toBigInt(512);
  const plaintext_modulus = ethers.toBigInt(10);
  const moduli = [
    ethers.toBigInt("0xffffee001"),
    ethers.toBigInt("0xffffc4001"),
  ];
  const encodedE3ProgramParams = abiCoder.encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );
  const data = "0xda7a";
  const proof = "0x1337";

  async function setupOperatorForSortition(
    operator: Signer,
    bondingRegistry: any,
    licenseToken: any,
    usdcToken: any,
    ticketToken: any,
    registry: any,
  ): Promise<void> {
    const operatorAddress = await operator.getAddress();
    await licenseToken.mintAllocation(
      operatorAddress,
      ethers.parseEther("10000"),
      "Test allocation",
    );
    await usdcToken.mint(operatorAddress, ethers.parseUnits("100000", 6));
    await licenseToken
      .connect(operator)
      .approve(await bondingRegistry.getAddress(), ethers.parseEther("2000"));
    await bondingRegistry
      .connect(operator)
      .bondLicense(ethers.parseEther("1000"));
    await bondingRegistry.connect(operator).registerOperator();
    const ticketAmount = ethers.parseUnits("100", 6);
    await usdcToken
      .connect(operator)
      .approve(await ticketToken.getAddress(), ticketAmount);
    await bondingRegistry.connect(operator).addTicketBalance(ticketAmount);
    await registry.addCiphernode(operatorAddress);
  }

  const setupAndPublishCommittee = async (
    registry: any,
    e3Id: number,
    nodes: string[],
    publicKey: string,
    operators: Signer[],
  ): Promise<void> => {
    for (const operator of operators) {
      await registry.connect(operator).submitTicket(e3Id, 1);
    }
    await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
    await registry.finalizeCommittee(e3Id);
    const pkCommitment = ethers.keccak256(publicKey);
    await registry.publishCommittee(e3Id, nodes, publicKey, pkCommitment, "0x");
  };

  const setup = async () => {
    const [owner, notTheOwner, operator1, operator2, operator3, treasury] =
      await ethers.getSigners();
    const ownerAddress = await owner.getAddress();
    const treasuryAddress = await treasury.getAddress();

    const { mockUSDC } = await ignition.deploy(MockStableTokenModule, {
      parameters: { MockUSDC: { initialSupply: 1_000_000 } },
    });
    const usdcToken = MockUSDCFactory.connect(
      await mockUSDC.getAddress(),
      owner,
    );

    const { enclaveToken: licenseToken } = await ignition.deploy(
      EnclaveTokenModule,
      { parameters: { EnclaveToken: { owner: ownerAddress } } },
    );
    const { enclaveTicketToken: ticketToken } = await ignition.deploy(
      EnclaveTicketTokenModule,
      {
        parameters: {
          EnclaveTicketToken: {
            baseToken: await usdcToken.getAddress(),
            registry: addressOne,
            owner: ownerAddress,
          },
        },
      },
    );

    const { slashingManager } = await ignition.deploy(SlashingManagerModule, {
      parameters: { SlashingManager: { admin: ownerAddress } },
    });
    const { cipherNodeRegistry } = await ignition.deploy(
      CiphernodeRegistryModule,
      {
        parameters: {
          CiphernodeRegistry: {
            owner: ownerAddress,
            submissionWindow: SORTITION_SUBMISSION_WINDOW,
          },
        },
      },
    );
    const ciphernodeRegistryAddress = await cipherNodeRegistry.getAddress();
    const ciphernodeRegistryContract = CiphernodeRegistryOwnableFactory.connect(
      ciphernodeRegistryAddress,
      owner,
    );

    const { bondingRegistry: _bondingRegistry } = await ignition.deploy(
      BondingRegistryModule,
      {
        parameters: {
          BondingRegistry: {
            owner: ownerAddress,
            ticketToken: await ticketToken.getAddress(),
            licenseToken: await licenseToken.getAddress(),
            registry: ciphernodeRegistryAddress,
            slashedFundsTreasury: ownerAddress,
            ticketPrice: ethers.parseUnits("10", 6),
            licenseRequiredBond: ethers.parseEther("1000"),
            minTicketBalance: 5,
            exitDelay: 7 * 24 * 60 * 60,
          },
        },
      },
    );
    const bondingRegistry = BondingRegistryFactory.connect(
      await _bondingRegistry.getAddress(),
      owner,
    );

    const { enclave: _enclave } = await ignition.deploy(EnclaveModule, {
      parameters: {
        Enclave: {
          owner: ownerAddress,
          maxDuration: THIRTY_DAYS_IN_SECONDS,
          registry: ciphernodeRegistryAddress,
          bondingRegistry: await bondingRegistry.getAddress(),
          e3RefundManager: addressOne,
          feeToken: await usdcToken.getAddress(),
          timeoutConfig,
        },
      },
    });
    const enclaveAddress = await _enclave.getAddress();
    const enclave = EnclaveFactory.connect(enclaveAddress, owner);

    const { e3RefundManager } = await ignition.deploy(E3RefundManagerModule, {
      parameters: {
        E3RefundManager: {
          owner: ownerAddress,
          enclave: enclaveAddress,
          treasury: treasuryAddress,
        },
      },
    });
    await enclave.setE3RefundManager(await e3RefundManager.getAddress());

    // Wire up
    await ciphernodeRegistryContract.setEnclave(enclaveAddress);
    await ciphernodeRegistryContract.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );
    await ticketToken.setRegistry(await bondingRegistry.getAddress());
    await bondingRegistry.setSlashingManager(
      await slashingManager.getAddress(),
    );
    await bondingRegistry.setRewardDistributor(enclaveAddress);
    await slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );

    // Mocks
    const { mockComputeProvider } = await ignition.deploy(
      mockComputeProviderModule,
    );
    const { mockDecryptionVerifier: decryptionVerifier } =
      await ignition.deploy(MockDecryptionVerifierModule);
    const { mockE3Program: e3Program } =
      await ignition.deploy(MockE3ProgramModule);
    const { mockPkVerifier: pkVerifier } =
      await ignition.deploy(MockPkVerifierModule);

    await enclave.enableE3Program(await e3Program.getAddress());
    await enclave.setParamSet(0, encodedE3ProgramParams);
    await enclave.setDecryptionVerifier(
      encryptionSchemeId,
      await decryptionVerifier.getAddress(),
    );
    await enclave.setPkVerifier(
      encryptionSchemeId,
      await pkVerifier.getAddress(),
    );

    // Operators
    await licenseToken.setTransferRestriction(false);
    for (const operator of [operator1, operator2, operator3]) {
      await setupOperatorForSortition(
        operator,
        bondingRegistry,
        licenseToken,
        usdcToken,
        ticketToken,
        ciphernodeRegistryContract,
      );
    }
    await mine(1);

    // Mint USDC
    const mintAmount = ethers.parseUnits("1000000", 6);
    await usdcToken.mint(ownerAddress, mintAmount);
    await usdcToken.mint(await notTheOwner.getAddress(), mintAmount);

    // Committee Thresholds: Micro [1,3], Small [2,5]
    await enclave.setCommitteeThresholds(0, [1, 3]);
    await enclave.setCommitteeThresholds(1, [2, 5]);

    // Build request params
    const now = await time.latest();
    const request = {
      committeeSize: 0, // Micro
      inputWindow: [now + 10, now + inputWindowDuration] as [number, number],
      e3Program: await e3Program.getAddress(),
      paramSet: 0,
      computeProviderParams: abiCoder.encode(
        ["address"],
        [await decryptionVerifier.getAddress()],
      ),
      customParams: abiCoder.encode(
        ["address"],
        ["0x1234567890123456789012345678901234567890"],
      ),
      proofAggregationEnabled: false,
    };

    return {
      owner,
      notTheOwner,
      operator1,
      operator2,
      operator3,
      treasury,
      enclave,
      ciphernodeRegistryContract,
      bondingRegistry,
      licenseToken,
      ticketToken,
      usdcToken,
      slashingManager,
      e3RefundManager,
      request,
      mocks: { decryptionVerifier, e3Program, mockComputeProvider, pkVerifier },
    };
  };

  // ──────────────────────────────────────────────────────────────────────────
  //  getE3Quote() — Parametric Fee Calculation
  // ──────────────────────────────────────────────────────────────────────────

  describe("getE3Quote()", function () {
    it("returns a fee based on BaseCosts, committee size, and duration", async function () {
      const { enclave, request } = await loadFixture(setup);

      const fee = await enclave.getE3Quote(request);
      // Fee must be > 0 with default baseCosts
      expect(fee).to.be.gt(0);
    });

    it("computes fee correctly using the parametric formula", async function () {
      const { enclave, request, ciphernodeRegistryContract } =
        await loadFixture(setup);

      // Get the resolved threshold for Micro (committeeSize = 0) → [1, 3]
      const n = 3n; // total committee
      const m = 1n; // quorum

      // Get pricing config
      const pc = await enclave.getPricingConfig();

      // Get timeout config
      const config = await enclave.getTimeoutConfig();
      const sortitionWindow =
        await ciphernodeRegistryContract.sortitionSubmissionWindow();
      const duration =
        sortitionWindow +
        BigInt(request.inputWindow[1] - request.inputWindow[0]) +
        (config.dkgWindow * BigInt(pc.dkgUtilizationBps)) / 10000n +
        (config.computeWindow * BigInt(pc.computeUtilizationBps)) / 10000n +
        (config.decryptionWindow * BigInt(pc.decryptUtilizationBps)) / 10000n;

      // Calculate expected fee (proof-aware)
      const proofsPerNode = 6n + 2n * (n - 1n) * 2n;
      let baseFee = pc.keyGenFixedPerNode * n;
      baseFee += pc.keyGenPerEncryptionProof * n * proofsPerNode;
      if (n > 1n) baseFee += (pc.coordinationPerPair * n * (n - 1n)) / 2n;
      baseFee += pc.verificationPerProof * n * proofsPerNode;
      baseFee += pc.availabilityPerNodePerSec * n * duration;
      baseFee += pc.decryptionPerNode * m;
      if (m > 1n) baseFee += (pc.coordinationPerPair * m * (m - 1n)) / 2n;
      baseFee += pc.publicationBase;

      const marginBps = pc.marginBps;
      const expectedFee = (baseFee * (10000n + BigInt(marginBps))) / 10000n;

      const actualFee = await enclave.getE3Quote(request);
      expect(actualFee).to.equal(expectedFee);
    });

    it("fee increases with larger committee size", async function () {
      const { enclave, request } = await loadFixture(setup);

      const microFee = await enclave.getE3Quote(request);

      // Build request with Small committee (larger)
      const now = await time.latest();
      const smallRequest = {
        ...request,
        committeeSize: 1, // Small → [2, 5]
        inputWindow: [now + 10, now + inputWindowDuration] as [number, number],
      };
      const smallFee = await enclave.getE3Quote(smallRequest);

      expect(smallFee).to.be.gt(microFee);
    });

    it("fee increases with longer input window", async function () {
      const { enclave, request } = await loadFixture(setup);

      const shortFee = await enclave.getE3Quote(request);

      const now = await time.latest();
      const longRequest = {
        ...request,
        inputWindow: [now + 10, now + 3600] as [number, number], // 1 hour vs 5min
      };
      const longFee = await enclave.getE3Quote(longRequest);

      expect(longFee).to.be.gt(shortFee);
    });

    it("fee reflects margin changes", async function () {
      const { enclave, request } = await loadFixture(setup);

      const fee10Pct = await enclave.getE3Quote(request);

      // Set margin to 20%
      const pc = toPlainConfig(await enclave.getPricingConfig());
      await enclave.setPricingConfig({ ...pc, marginBps: 2000 });
      const fee20Pct = await enclave.getE3Quote(request);

      expect(fee20Pct).to.be.gt(fee10Pct);

      // Set margin to 0%
      const pc2 = toPlainConfig(await enclave.getPricingConfig());
      await enclave.setPricingConfig({ ...pc2, marginBps: 0 });
      const feeZero = await enclave.getE3Quote(request);

      expect(feeZero).to.be.lt(fee10Pct);
    });
  });

  // ──────────────────────────────────────────────────────────────────────────
  //  setPricingConfig() — Governance
  // ──────────────────────────────────────────────────────────────────────────

  describe("setPricingConfig()", function () {
    it("reverts if not called by owner", async function () {
      const { enclave, notTheOwner } = await loadFixture(setup);
      await expect(
        enclave.connect(notTheOwner).setPricingConfig(defaultPricingConfig),
      ).to.be.revertedWithCustomError(enclave, "OwnableUnauthorizedAccount");
    });

    it("updates config and emits event", async function () {
      const { enclave } = await loadFixture(setup);
      const newConfig = {
        ...defaultPricingConfig,
        keyGenFixedPerNode: 100000n,
        keyGenPerEncryptionProof: 50000n,
        coordinationPerPair: 10000n,
        availabilityPerNodePerSec: 40n,
        decryptionPerNode: 300000n,
        publicationBase: 1000000n,
      };

      await expect(enclave.setPricingConfig(newConfig)).to.emit(
        enclave,
        "PricingConfigUpdated",
      );

      const stored = await enclave.getPricingConfig();
      expect(stored.keyGenFixedPerNode).to.equal(100000n);
      expect(stored.keyGenPerEncryptionProof).to.equal(50000n);
      expect(stored.coordinationPerPair).to.equal(10000n);
      expect(stored.availabilityPerNodePerSec).to.equal(40n);
      expect(stored.decryptionPerNode).to.equal(300000n);
      expect(stored.publicationBase).to.equal(1000000n);
    });

    it("changes the fee returned by getE3Quote", async function () {
      const { enclave, request } = await loadFixture(setup);

      const feeBefore = await enclave.getE3Quote(request);

      // Double base costs
      await enclave.setPricingConfig({
        ...defaultPricingConfig,
        keyGenFixedPerNode: 100000n,
        keyGenPerEncryptionProof: 50000n,
        coordinationPerPair: 10000n,
        availabilityPerNodePerSec: 40n,
        decryptionPerNode: 300000n,
        publicationBase: 1000000n,
      });

      const feeAfter = await enclave.getE3Quote(request);
      expect(feeAfter).to.be.gt(feeBefore);
    });

    it("reverts if margin exceeds 100%", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(
        enclave.setPricingConfig({ ...defaultPricingConfig, marginBps: 10001 }),
      ).to.be.revertedWithCustomError(enclave, "BpsExceedsMax");
    });

    it("allows setting margin to 0", async function () {
      const { enclave } = await loadFixture(setup);
      await enclave.setPricingConfig({ ...defaultPricingConfig, marginBps: 0 });
      const pc = await enclave.getPricingConfig();
      expect(pc.marginBps).to.equal(0);
    });

    it("reverts if protocolShareBps exceeds 100%", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(
        enclave.setPricingConfig({
          ...defaultPricingConfig,
          protocolShareBps: 10001,
        }),
      ).to.be.revertedWithCustomError(enclave, "BpsExceedsMax");
    });

    it("reverts if minCommitteeSize < minThreshold", async function () {
      const { enclave } = await loadFixture(setup);
      await expect(
        enclave.setPricingConfig({
          ...defaultPricingConfig,
          minCommitteeSize: 2,
          minThreshold: 5,
        }),
      ).to.be.revertedWithCustomError(enclave, "MinSizeBelowMinThreshold");
    });

    it("enforces bounds on setCommitteeThresholds", async function () {
      const { enclave } = await loadFixture(setup);

      // Set minimum bounds via pricing config
      await enclave.setPricingConfig({
        ...defaultPricingConfig,
        minCommitteeSize: 5,
        minThreshold: 3,
      });

      // Should fail: committee size 4 < min 5
      await expect(
        enclave.setCommitteeThresholds(0, [3, 4]),
      ).to.be.revertedWithCustomError(enclave, "BelowMinCommitteeSize");

      // Should fail: threshold 2 < min 3
      await expect(
        enclave.setCommitteeThresholds(0, [2, 6]),
      ).to.be.revertedWithCustomError(enclave, "BelowMinThreshold");

      // Should succeed: meets both minimums
      await expect(enclave.setCommitteeThresholds(0, [3, 5])).to.not.be.revert(
        ethers,
      );
    });
  });

  // ──────────────────────────────────────────────────────────────────────────
  //  Protocol Treasury Share on Reward Distribution
  // ──────────────────────────────────────────────────────────────────────────

  describe("Protocol treasury share on success", function () {
    it("sends 100% to CNs when protocolShareBps is 0 (default)", async function () {
      const {
        enclave,
        usdcToken,
        ciphernodeRegistryContract,
        operator1,
        operator2,
        operator3,
        mocks: { decryptionVerifier, e3Program },
      } = await loadFixture(setup);

      // Build a fresh request with current timestamps
      const now = await time.latest();
      const freshRequest = {
        committeeSize: 0,
        inputWindow: [now + 100, now + inputWindowDuration + 100] as [
          number,
          number,
        ],
        e3Program: await e3Program.getAddress(),
        paramSet: 0,
        computeProviderParams: abiCoder.encode(
          ["address"],
          [await decryptionVerifier.getAddress()],
        ),
        customParams: abiCoder.encode(
          ["address"],
          ["0x1234567890123456789012345678901234567890"],
        ),
        proofAggregationEnabled: false,
      };

      // Make request with large approval to avoid fee mismatch
      await usdcToken.approve(await enclave.getAddress(), ethers.MaxUint256);
      await enclave.request(freshRequest);
      const e3Id = 0;
      const fee = await enclave.e3Payments(e3Id);

      // Setup committee
      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
        await operator3.getAddress(),
      ];
      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        nodes,
        "0x1234",
        [operator1, operator2, operator3],
      );

      // Publish ciphertext
      await time.increase(inputWindowDuration + 200);
      await enclave.publishCiphertextOutput(e3Id, data, proof);

      // Record operator balances before distribution
      const op1Before = await usdcToken.balanceOf(nodes[0]);
      const op2Before = await usdcToken.balanceOf(nodes[1]);
      const op3Before = await usdcToken.balanceOf(nodes[2]);

      // Publish plaintext (triggers _distributeRewards)
      await enclave.publishPlaintextOutput(e3Id, data, proof);

      const op1After = await usdcToken.balanceOf(nodes[0]);
      const op2After = await usdcToken.balanceOf(nodes[1]);
      const op3After = await usdcToken.balanceOf(nodes[2]);

      // All fee distributed to operators (100%, no protocol share)
      const totalDistributed =
        op1After - op1Before + (op2After - op2Before) + (op3After - op3Before);
      expect(totalDistributed).to.equal(fee);
    });

    it("splits fee between CNs and treasury when protocolShareBps > 0", async function () {
      const {
        enclave,
        usdcToken,
        ciphernodeRegistryContract,
        treasury,
        operator1,
        operator2,
        operator3,
        mocks: { decryptionVerifier, e3Program },
      } = await loadFixture(setup);

      // Configure 20% protocol share
      const treasuryAddr = await treasury.getAddress();
      await enclave.setPricingConfig({
        ...defaultPricingConfig,
        protocolTreasury: treasuryAddr,
        protocolShareBps: 2000,
      });

      // Build a fresh request with current timestamps
      const now = await time.latest();
      const freshRequest = {
        committeeSize: 0,
        inputWindow: [now + 100, now + inputWindowDuration + 100] as [
          number,
          number,
        ],
        e3Program: await e3Program.getAddress(),
        paramSet: 0,
        computeProviderParams: abiCoder.encode(
          ["address"],
          [await decryptionVerifier.getAddress()],
        ),
        customParams: abiCoder.encode(
          ["address"],
          ["0x1234567890123456789012345678901234567890"],
        ),
        proofAggregationEnabled: false,
      };

      // Make request with large approval
      await usdcToken.approve(await enclave.getAddress(), ethers.MaxUint256);
      await enclave.request(freshRequest);
      const e3Id = 0;
      const fee = await enclave.e3Payments(e3Id);

      // Setup committee
      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
        await operator3.getAddress(),
      ];
      await setupAndPublishCommittee(
        ciphernodeRegistryContract,
        e3Id,
        nodes,
        "0x1234",
        [operator1, operator2, operator3],
      );

      // Publish outputs
      await time.increase(inputWindowDuration + 200);
      await enclave.publishCiphertextOutput(e3Id, data, proof);

      const treasuryBefore = await usdcToken.balanceOf(treasuryAddr);
      const op1Before = await usdcToken.balanceOf(nodes[0]);
      const op2Before = await usdcToken.balanceOf(nodes[1]);
      const op3Before = await usdcToken.balanceOf(nodes[2]);

      await enclave.publishPlaintextOutput(e3Id, data, proof);

      const treasuryAfter = await usdcToken.balanceOf(treasuryAddr);
      const op1After = await usdcToken.balanceOf(nodes[0]);
      const op2After = await usdcToken.balanceOf(nodes[1]);
      const op3After = await usdcToken.balanceOf(nodes[2]);

      const expectedProtocol = (fee * 2000n) / 10000n;
      const expectedCN = fee - expectedProtocol;
      const totalOpDistributed =
        op1After - op1Before + (op2After - op2Before) + (op3After - op3Before);

      expect(treasuryAfter - treasuryBefore).to.equal(expectedProtocol);
      expect(totalOpDistributed).to.equal(expectedCN);
    });
  });

  // ──────────────────────────────────────────────────────────────────────────
  //  Default Pricing Parameters (set in initialize)
  // ──────────────────────────────────────────────────────────────────────────

  describe("Default pricing parameters", function () {
    it("has correct default pricing config from initialize", async function () {
      const { enclave } = await loadFixture(setup);
      const pc = await enclave.getPricingConfig();
      expect(pc.keyGenFixedPerNode).to.equal(50000);
      expect(pc.keyGenPerEncryptionProof).to.equal(25000);
      expect(pc.coordinationPerPair).to.equal(5000);
      expect(pc.availabilityPerNodePerSec).to.equal(20);
      expect(pc.decryptionPerNode).to.equal(150000);
      expect(pc.publicationBase).to.equal(500000);
      expect(pc.verificationPerProof).to.equal(2000);
      expect(pc.marginBps).to.equal(1000);
      expect(pc.protocolShareBps).to.equal(0);
      expect(pc.dkgUtilizationBps).to.equal(2500);
      expect(pc.computeUtilizationBps).to.equal(5000);
      expect(pc.decryptUtilizationBps).to.equal(2500);
      expect(pc.protocolTreasury).to.equal(ethers.ZeroAddress);
      expect(pc.minCommitteeSize).to.equal(0);
      expect(pc.minThreshold).to.equal(0);
    });
  });

  // ──────────────────────────────────────────────────────────────────────────
  //  E3 Request with Parametric Pricing (end-to-end)
  // ──────────────────────────────────────────────────────────────────────────

  describe("End-to-end request with parametric pricing", function () {
    it("charges the computed fee and completes successfully", async function () {
      const { enclave, usdcToken, request, owner } = await loadFixture(setup);

      const fee = await enclave.getE3Quote(request);
      const ownerAddr = await owner.getAddress();
      const balanceBefore = await usdcToken.balanceOf(ownerAddr);

      await usdcToken.approve(await enclave.getAddress(), fee);
      await enclave.request(request);

      const balanceAfter = await usdcToken.balanceOf(ownerAddr);
      expect(balanceBefore - balanceAfter).to.equal(fee);
    });

    it("reverts if USDC allowance is less than computed fee", async function () {
      const { enclave, usdcToken, request } = await loadFixture(setup);

      // Approve only 1 unit
      await usdcToken.approve(await enclave.getAddress(), 1);

      await expect(enclave.request(request)).to.be.revertedWithCustomError(
        usdcToken,
        "ERC20InsufficientAllowance",
      );
    });
  });
});
