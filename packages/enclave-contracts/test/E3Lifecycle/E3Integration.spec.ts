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
import MockDecryptionVerifierModule from "../../ignition/modules/mockDecryptionVerifier";
import MockE3ProgramModule from "../../ignition/modules/mockE3Program";
import MockCircuitVerifierModule from "../../ignition/modules/mockSlashingVerifier";
import MockStableTokenModule from "../../ignition/modules/mockStableToken";
import SlashingManagerModule from "../../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryOwnableFactory,
  E3RefundManager__factory as E3RefundManagerFactory,
  Enclave__factory as EnclaveFactory,
  EnclaveToken__factory as EnclaveTokenFactory,
  MockCircuitVerifier__factory as MockCircuitVerifierFactory,
  MockDecryptionVerifier__factory as MockDecryptionVerifierFactory,
  MockE3Program__factory as MockE3ProgramFactory,
  MockUSDC__factory as MockUSDCFactory,
  SlashingManager__factory as SlashingManagerFactory,
} from "../../types";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture, time } = networkHelpers;

/**
 * Integration tests for E3 Refund/Timeout Mechanism
 *
 * These tests verify the full integration between:
 * - Enclave.sol (main coordinator with integrated lifecycle management)
 * - E3RefundManager.sol (refund calculation and claiming)
 * - CiphernodeRegistryOwnable.sol (committee management)
 */
describe("E3 Integration - Refund/Timeout Mechanism", function () {
  // Time constants
  const ONE_HOUR = 60 * 60;
  const ONE_DAY = 24 * ONE_HOUR;
  const THREE_DAYS = 3 * ONE_DAY;
  const SEVEN_DAYS = 7 * ONE_DAY;
  const THIRTY_DAYS = 30 * ONE_DAY;
  const SORTITION_SUBMISSION_WINDOW = 10;

  const addressOne = "0x0000000000000000000000000000000000000001";

  // Default timeout configuration
  const defaultTimeoutConfig = {
    committeeFormationWindow: ONE_DAY,
    dkgWindow: ONE_DAY,
    computeWindow: THREE_DAYS,
    decryptionWindow: ONE_DAY,
  };

  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  const polynomial_degree = ethers.toBigInt(2048);
  const plaintext_modulus = ethers.toBigInt(1032193);
  const moduli = [ethers.toBigInt("18014398492704769")];

  const encodedE3ProgramParams = abiCoder.encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );

  const encryptionSchemeId =
    "0x2c2a814a0495f913a3a312fc4771e37552bc14f8a2d4075a08122d356f0849c6";

  // Slash-related constants for E2E tests
  const REASON_BAD_PROOF = ethers.keccak256(ethers.toUtf8Bytes("E3_BAD_PROOF"));
  const PROOF_PAYLOAD_TYPEHASH = ethers.keccak256(
    ethers.toUtf8Bytes(
      "ProofPayload(uint256 chainId,uint256 e3Id,uint256 proofType,bytes zkProof,bytes publicSignals)",
    ),
  );

  /**
   * Helper to create a signed proof evidence bundle for proposeSlash.
   */
  async function signAndEncodeProof(
    signer: Signer,
    e3Id: number,
    verifierAddress: string,
    zkProof: string = "0x1234",
    publicInputs: string[] = [ethers.ZeroHash],
    chainId: number = 31337,
    proofType: number = 0,
  ): Promise<string> {
    const messageHash = ethers.keccak256(
      abiCoder.encode(
        ["bytes32", "uint256", "uint256", "uint256", "bytes32", "bytes32"],
        [
          PROOF_PAYLOAD_TYPEHASH,
          chainId,
          e3Id,
          proofType,
          ethers.keccak256(zkProof),
          ethers.keccak256(
            ethers.solidityPacked(["bytes32[]"], [publicInputs]),
          ),
        ],
      ),
    );
    const signature = await signer.signMessage(ethers.getBytes(messageHash));
    return abiCoder.encode(
      ["bytes", "bytes32[]", "bytes", "uint256", "uint256", "address"],
      [zkProof, publicInputs, signature, chainId, proofType, verifierAddress],
    );
  }

  const setup = async () => {
    // ── Signers ────────────────────────────────────────────────────────────────
    const [owner, requester, treasury, operator1, operator2, computeProvider] =
      await ethers.getSigners();

    const ownerAddress = await owner.getAddress();
    const treasuryAddress = await treasury.getAddress();
    const requesterAddress = await requester.getAddress();

    // ── Token Contracts ────────────────────────────────────────────────────────
    const { mockUSDC } = await ignition.deploy(MockStableTokenModule, {
      parameters: { MockUSDC: { initialSupply: 10_000_000 } },
    });
    const usdcToken = MockUSDCFactory.connect(
      await mockUSDC.getAddress(),
      owner,
    );

    const { enclaveToken } = await ignition.deploy(EnclaveTokenModule, {
      parameters: { EnclaveToken: { owner: ownerAddress } },
    });
    const enclToken = EnclaveTokenFactory.connect(
      await enclaveToken.getAddress(),
      owner,
    );

    const { enclaveTicketToken } = await ignition.deploy(
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

    // ── Registry & Slashing ────────────────────────────────────────────────────
    const { slashingManager } = await ignition.deploy(SlashingManagerModule, {
      parameters: {
        SlashingManager: {
          admin: ownerAddress,
        },
      },
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
    const registry = CiphernodeRegistryOwnableFactory.connect(
      ciphernodeRegistryAddress,
      owner,
    );

    const { bondingRegistry: _bondingRegistry } = await ignition.deploy(
      BondingRegistryModule,
      {
        parameters: {
          BondingRegistry: {
            owner: ownerAddress,
            ticketToken: await enclaveTicketToken.getAddress(),
            licenseToken: await enclToken.getAddress(),
            registry: ciphernodeRegistryAddress,
            slashedFundsTreasury: treasuryAddress,
            ticketPrice: ethers.parseUnits("10", 6),
            licenseRequiredBond: ethers.parseEther("1000"),
            minTicketBalance: 5,
            exitDelay: SEVEN_DAYS,
          },
        },
      },
    );
    const bondingRegistry = BondingRegistryFactory.connect(
      await _bondingRegistry.getAddress(),
      owner,
    );

    // ── Enclave ────────────────────────────────────────────────────────────────
    const { enclave: _enclave } = await ignition.deploy(EnclaveModule, {
      parameters: {
        Enclave: {
          params: encodedE3ProgramParams,
          owner: ownerAddress,
          maxDuration: THIRTY_DAYS,
          registry: ciphernodeRegistryAddress,
          bondingRegistry: await bondingRegistry.getAddress(),
          e3RefundManager: addressOne, // updated below
          feeToken: await usdcToken.getAddress(),
          timeoutConfig: defaultTimeoutConfig,
        },
      },
    });
    const enclaveAddress = await _enclave.getAddress();
    const enclave = EnclaveFactory.connect(enclaveAddress, owner);

    const { e3RefundManager: _e3RefundManager } = await ignition.deploy(
      E3RefundManagerModule,
      {
        parameters: {
          E3RefundManager: {
            owner: ownerAddress,
            enclave: enclaveAddress,
            treasury: treasuryAddress,
          },
        },
      },
    );
    const e3RefundManagerAddress = await _e3RefundManager.getAddress();
    const e3RefundManager = E3RefundManagerFactory.connect(
      e3RefundManagerAddress,
      owner,
    );

    // ── Mock E3 Program & Decryption Verifier ──────────────────────────────────
    const { mockE3Program } = await ignition.deploy(MockE3ProgramModule, {
      parameters: { MockE3Program: { encryptionSchemeId } },
    });
    const e3Program = MockE3ProgramFactory.connect(
      await mockE3Program.getAddress(),
      owner,
    );

    const { mockDecryptionVerifier } = await ignition.deploy(
      MockDecryptionVerifierModule,
    );
    const decryptionVerifier = MockDecryptionVerifierFactory.connect(
      await mockDecryptionVerifier.getAddress(),
      owner,
    );

    // ── Mock Circuit Verifier (for SlashingManager proof-based slashes) ────────
    const { mockCircuitVerifier } = await ignition.deploy(
      MockCircuitVerifierModule,
    );
    const circuitVerifier = MockCircuitVerifierFactory.connect(
      await mockCircuitVerifier.getAddress(),
      owner,
    );

    // ── SlashingManager typed factory ──────────────────────────────────────────
    const slashingManagerTyped = SlashingManagerFactory.connect(
      await slashingManager.getAddress(),
      owner,
    );

    // ── Wire Up Contracts ──────────────────────────────────────────────────────
    await enclave.setE3RefundManager(e3RefundManagerAddress);
    await enclave.setSlashingManager(await slashingManager.getAddress());
    await enclave.enableE3Program(await e3Program.getAddress());
    await enclave.setDecryptionVerifier(
      encryptionSchemeId,
      await decryptionVerifier.getAddress(),
    );

    await bondingRegistry.setRewardDistributor(enclaveAddress);
    await bondingRegistry.setSlashingManager(
      await slashingManager.getAddress(),
    );

    await slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );
    await slashingManager.setCiphernodeRegistry(ciphernodeRegistryAddress);
    await slashingManager.setEnclave(enclaveAddress);
    await slashingManager.setE3RefundManager(e3RefundManagerAddress);

    await registry.setEnclave(enclaveAddress);
    await registry.setBondingRegistry(await bondingRegistry.getAddress());
    await registry.setSlashingManager(await slashingManager.getAddress());

    await enclaveTicketToken.setRegistry(await bondingRegistry.getAddress());

    // ── Slash Policy (for E2E routing tests) ───────────────────────────────────
    await slashingManagerTyped.setSlashPolicy(REASON_BAD_PROOF, {
      ticketPenalty: ethers.parseUnits("50", 6),
      licensePenalty: ethers.parseEther("100"),
      requiresProof: true,
      proofVerifier: await circuitVerifier.getAddress(),
      banNode: false,
      appealWindow: 0,
      enabled: true,
      affectsCommittee: false,
      failureReason: 0,
    });

    // ── Mint Tokens ────────────────────────────────────────────────────────────
    await usdcToken.mint(requesterAddress, ethers.parseUnits("10000", 6));
    await usdcToken.mint(e3RefundManagerAddress, ethers.parseUnits("10000", 6));

    // ── Helpers ────────────────────────────────────────────────────────────────
    const makeRequest = async (
      signer: Signer = requester,
    ): Promise<{ e3Id: number }> => {
      const startTime = (await time.latest()) + 100;

      const requestParams = {
        threshold: [2, 2] as [number, number],
        inputWindow: [startTime + 100, startTime + ONE_DAY] as [number, number],
        e3Program: await e3Program.getAddress(),
        e3ProgramParams: encodedE3ProgramParams,
        computeProviderParams: abiCoder.encode(
          ["address"],
          [await decryptionVerifier.getAddress()],
        ),
        customParams: abiCoder.encode(
          ["address"],
          ["0x1234567890123456789012345678901234567890"],
        ),
      };

      const fee = await enclave.getE3Quote(requestParams);
      await usdcToken.connect(signer).approve(enclaveAddress, fee);
      await enclave.connect(signer).request(requestParams);

      return { e3Id: 0 };
    };

    const setupOperator = async (operator: Signer) => {
      const operatorAddress = await operator.getAddress();
      const ticketTokenAddress = await bondingRegistry.ticketToken();
      const ticketAmount = ethers.parseUnits("100", 6);

      await enclToken.setTransferRestriction(false);
      await enclToken.mintAllocation(
        operatorAddress,
        ethers.parseEther("10000"),
        "Test allocation",
      );
      await usdcToken.mint(operatorAddress, ethers.parseUnits("100000", 6));

      await enclToken
        .connect(operator)
        .approve(await bondingRegistry.getAddress(), ethers.parseEther("2000"));
      await bondingRegistry
        .connect(operator)
        .bondLicense(ethers.parseEther("1000"));
      await bondingRegistry.connect(operator).registerOperator();

      await usdcToken
        .connect(operator)
        .approve(ticketTokenAddress, ticketAmount);
      await bondingRegistry.connect(operator).addTicketBalance(ticketAmount);
    };

    // ── Return ─────────────────────────────────────────────────────────────────
    return {
      enclave,
      e3RefundManager,
      bondingRegistry,
      registry,
      slashingManager: slashingManagerTyped,
      circuitVerifier,
      usdcToken,
      enclToken,
      e3Program,
      decryptionVerifier,
      owner,
      requester,
      treasury,
      operator1,
      operator2,
      computeProvider,
      makeRequest,
      setupOperator,
    };
  };

  describe("E3 Request with Lifecycle Integration", function () {
    it("initializes E3 lifecycle when request is made", async function () {
      const {
        enclave,
        makeRequest,
        requester,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest();

      // Check that E3 lifecycle was initialized
      const stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(1); // E3Stage.Requested

      // Check requester is tracked
      const storedRequester = await enclave.getRequester(0);
      expect(storedRequester).to.equal(await requester.getAddress());
    });
  });

  describe("Committee Formed Integration", function () {
    it("transitions to CommitteeFormed when publishCommittee is called", async function () {
      const {
        enclave,
        registry,
        makeRequest,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      // Make a request first
      await makeRequest();

      // Verify stage is Requested
      let stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(1); // E3Stage.Requested

      // Submit tickets for sortition
      await registry.connect(operator1).submitTicket(0, 1);
      await registry.connect(operator2).submitTicket(0, 1);

      // Fast forward past submission window
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);

      // Finalize committee
      await registry.finalizeCommittee(0);

      // Publish committee (this triggers onCommitteePublished -> onCommitteeFormed)
      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
      ];
      const publicKey = "0x1234567890abcdef1234567890abcdef";
      const publicKeyHash = ethers.keccak256(publicKey);

      await registry.publishCommittee(0, nodes, publicKey, publicKeyHash);

      // Verify stage transitioned to KeyPublished (after publishCommittee which calls onKeyPublished)
      stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(3); // E3Stage.KeyPublished

      // Verify deadlines were set
      const deadlines = await enclave.getDeadlines(0);
      expect(deadlines.dkgDeadline).to.be.gt(0);
    });

    it("emits CommitteeFormed event when committee is published", async function () {
      const {
        enclave,
        registry,
        makeRequest,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      // Make a request
      await makeRequest();

      // Complete sortition process
      await registry.connect(operator1).submitTicket(0, 1);
      await registry.connect(operator2).submitTicket(0, 1);
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(0);

      // Publish committee and expect CommitteeFormed event
      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
      ];
      const publicKey = "0x1234567890abcdef1234567890abcdef";
      const publicKeyHash = ethers.keccak256(publicKey);

      await expect(
        registry.publishCommittee(0, nodes, publicKey, publicKeyHash),
      )
        .to.emit(enclave, "CommitteeFormed")
        .withArgs(0);
    });
  });

  describe("processE3Failure()", function () {
    it("reverts if lifecycle is not a valid contract", async function () {
      const {
        enclave,
        owner,
        makeRequest,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest();

      // Create a new enclave with addressOne as lifecycle placeholder (not a real contract)
      const newEnclaveContract = await ignition.deploy(EnclaveModule, {
        parameters: {
          Enclave: {
            params: encodedE3ProgramParams,
            owner: await owner.getAddress(),
            maxDuration: THIRTY_DAYS,
            registry: await enclave.ciphernodeRegistry(),
            bondingRegistry: await enclave.bondingRegistry(),
            e3RefundManager: addressOne,
            feeToken: await enclave.feeToken(),
          },
        },
      });
      const newEnclave = EnclaveFactory.connect(
        await newEnclaveContract.enclave.getAddress(),
        owner,
      );

      // Calling processE3Failure with a placeholder lifecycle should revert
      // (it will try to call getE3Stage on an EOA which will fail)
      await expect(newEnclave.processE3Failure(0)).to.be.revert(ethers);
    });

    it("reverts if E3 not in failed state", async function () {
      const { enclave, makeRequest, operator1, operator2, setupOperator } =
        await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest();

      // E3 is in Requested state, not Failed
      await expect(enclave.processE3Failure(0)).to.be.revertedWith(
        "E3 not failed",
      );
    });

    it("processes failure and calculates refund for committee formation timeout", async function () {
      const {
        enclave,
        e3RefundManager,
        makeRequest,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest();

      // Fast forward past committee formation deadline
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);

      // Mark E3 as failed
      await enclave.markE3Failed(0);

      const stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(6); // E3Stage.Failed

      // Process the failure
      await expect(enclave.processE3Failure(0)).to.emit(
        enclave,
        "E3FailureProcessed",
      );

      const distribution = await e3RefundManager.getRefundDistribution(0);
      expect(distribution.calculated).to.be.true;
      expect(distribution.requesterAmount).to.be.gt(0);
    });

    it("allows requester to claim refund after failure processing", async function () {
      const {
        enclave,
        e3RefundManager,
        makeRequest,
        requester,
        usdcToken,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest();

      // Get initial balance
      const balanceBefore = await usdcToken.balanceOf(
        await requester.getAddress(),
      );

      // Fast forward and fail E3
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(0);
      await enclave.processE3Failure(0);

      // Claim refund
      await e3RefundManager.connect(requester).claimRequesterRefund(0);

      const balanceAfter = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      expect(balanceAfter).to.be.gt(balanceBefore);
    });

    it("reverts if trying to process failure twice", async function () {
      const { enclave, makeRequest, operator1, operator2, setupOperator } =
        await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest();

      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(0);
      await enclave.processE3Failure(0);

      // Second call should fail - payment already cleared
      await expect(enclave.processE3Failure(0)).to.be.revertedWith(
        "No payment to refund",
      );
    });

    it("reverts if requester tries to claim refund twice", async function () {
      const {
        enclave,
        e3RefundManager,
        makeRequest,
        requester,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest();

      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(0);
      await enclave.processE3Failure(0);

      // First claim succeeds
      await e3RefundManager.connect(requester).claimRequesterRefund(0);

      // Second claim should fail
      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "AlreadyClaimed");
    });

    it("reverts if refund not yet calculated", async function () {
      const {
        e3RefundManager,
        makeRequest,
        requester,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest();

      // Try to claim before failure is processed
      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "RefundNotCalculated");
    });
  });

  describe("Slashed Funds Escrow", function () {
    it("E2E: slash via SlashingManager escrows actual USDC to refund manager and requester can claim", async function () {
      const {
        enclave,
        e3RefundManager,
        registry,
        slashingManager,
        circuitVerifier,
        bondingRegistry,
        usdcToken,
        makeRequest,
        requester,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      // 1. Request E3, form committee, publish key
      await makeRequest();
      await registry.connect(operator1).submitTicket(0, 1);
      await registry.connect(operator2).submitTicket(0, 1);
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(0);

      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
      ];
      const publicKey = "0x1234567890abcdef1234567890abcdef";
      const publicKeyHash = ethers.keccak256(publicKey);
      await registry.publishCommittee(0, nodes, publicKey, publicKeyHash);

      // 2. Wait past compute deadline → mark as failed
      const e3 = await enclave.getE3(0);
      const computeDeadline =
        Number(e3.inputWindow[1]) + defaultTimeoutConfig.computeWindow;
      await time.increaseTo(computeDeadline + 1);
      await enclave.markE3Failed(0);

      // 3. Process failure → distribution calculated, funds transferred to refund manager
      await enclave.processE3Failure(0);
      const distributionBefore = await e3RefundManager.getRefundDistribution(0);
      expect(distributionBefore.calculated).to.be.true;

      // Record refund manager USDC balance before slash routing
      const refundManagerBalanceBefore = await usdcToken.balanceOf(
        await e3RefundManager.getAddress(),
      );

      // Record BondingRegistry's slashedTicketBalance before slash
      const slashedBalanceBefore = await bondingRegistry.slashedTicketBalance();

      // 4. Slash operator1 via proposeSlash (Lane A) — real on-chain flow
      //    This triggers: _executeSlash → slashTicketBalance → redirectSlashedTicketFunds
      //    → ticketToken.payout(refundManager, amount) → enclave.escrowSlashedFunds → e3RefundManager.escrowSlashedFunds
      const proof = await signAndEncodeProof(
        operator1,
        0,
        await circuitVerifier.getAddress(),
      );

      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_PROOF,
        proof,
      );

      // 5. Verify actual USDC moved to the refund manager
      const refundManagerBalanceAfter = await usdcToken.balanceOf(
        await e3RefundManager.getAddress(),
      );
      const actualSlashedAmount =
        refundManagerBalanceAfter - refundManagerBalanceBefore;
      expect(actualSlashedAmount).to.be.gt(0);

      // Verify BondingRegistry's slashedTicketBalance was decremented
      const slashedBalanceAfter = await bondingRegistry.slashedTicketBalance();
      expect(slashedBalanceAfter).to.equal(
        slashedBalanceBefore, // slash added then redirect removed the same amount
      );

      // 6. Verify distribution was updated with requester-first priority
      const distributionAfter = await e3RefundManager.getRefundDistribution(0);
      expect(distributionAfter.totalSlashed).to.equal(actualSlashedAmount);
      expect(distributionAfter.requesterAmount).to.be.gte(
        distributionBefore.requesterAmount,
      );

      // 7. Verify requester can actually claim and receives the correct USDC
      const requesterBalanceBefore = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      await e3RefundManager.connect(requester).claimRequesterRefund(0);
      const requesterBalanceAfter = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      expect(requesterBalanceAfter - requesterBalanceBefore).to.equal(
        distributionAfter.requesterAmount,
      );
    });

    it("E2E: honest nodes can claim their share after slashed funds are escrowed", async function () {
      const {
        enclave,
        e3RefundManager,
        registry,
        slashingManager,
        circuitVerifier,
        usdcToken,
        makeRequest,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      // 1. Request E3, form committee, publish key
      await makeRequest();
      await registry.connect(operator1).submitTicket(0, 1);
      await registry.connect(operator2).submitTicket(0, 1);
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(0);

      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
      ];
      const publicKey = "0x1234567890abcdef1234567890abcdef";
      const publicKeyHash = ethers.keccak256(publicKey);
      await registry.publishCommittee(0, nodes, publicKey, publicKeyHash);

      // 2. Fail via compute timeout
      const e3 = await enclave.getE3(0);
      const computeDeadline =
        Number(e3.inputWindow[1]) + defaultTimeoutConfig.computeWindow;
      await time.increaseTo(computeDeadline + 1);
      await enclave.markE3Failed(0);
      await enclave.processE3Failure(0);

      // 3. Record distribution BEFORE slash to verify it actually changes
      const distributionBefore = await e3RefundManager.getRefundDistribution(0);
      const honestNodeAmountBefore = distributionBefore.honestNodeAmount;

      // 4. Slash operator1 — this routes funds into the refund pool
      const proof = await signAndEncodeProof(
        operator1,
        0,
        await circuitVerifier.getAddress(),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_PROOF,
        proof,
      );

      const distribution = await e3RefundManager.getRefundDistribution(0);
      expect(distribution.honestNodeCount).to.be.gt(0);
      // Verify that honestNodeAmount INCREASED due to slashed funds escrow
      expect(distribution.honestNodeAmount).to.be.gt(honestNodeAmountBefore);
      expect(distribution.totalSlashed).to.be.gt(0);

      // 5. operator2 (honest node) claims their share
      const op2BalanceBefore = await usdcToken.balanceOf(
        await operator2.getAddress(),
      );
      await e3RefundManager.connect(operator2).claimHonestNodeReward(0);
      const op2BalanceAfter = await usdcToken.balanceOf(
        await operator2.getAddress(),
      );

      const perNodeAmount =
        distribution.honestNodeAmount / BigInt(distribution.honestNodeCount);
      expect(op2BalanceAfter - op2BalanceBefore).to.equal(perNodeAmount);
    });

    it("requester-first priority: requester gets filled before honest nodes", async function () {
      const {
        enclave,
        e3RefundManager,
        makeRequest,
        owner,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest();

      // Fail the E3 at committee formation stage (no honest nodes, requester gets 95%)
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(0);
      await enclave.processE3Failure(0);

      const distributionBefore = await e3RefundManager.getRefundDistribution(0);
      const slashedAmount = ethers.parseUnits("100", 6);

      // requesterGap = originalPayment - requesterAmount (how much more needed to be whole)
      const requesterGap =
        distributionBefore.originalPayment - distributionBefore.requesterAmount;

      // Escrow slashed funds via the enclave proxy (swap enclave address for test)
      const originalEnclave = await e3RefundManager.enclave();
      await e3RefundManager.setEnclave(await owner.getAddress());
      await e3RefundManager.connect(owner).escrowSlashedFunds(0, slashedAmount);
      await e3RefundManager.setEnclave(originalEnclave);

      const distributionAfter = await e3RefundManager.getRefundDistribution(0);

      const expectedToRequester =
        slashedAmount >= requesterGap ? requesterGap : slashedAmount;
      const expectedToHonestNodes = slashedAmount - expectedToRequester;

      expect(distributionAfter.requesterAmount).to.equal(
        distributionBefore.requesterAmount + expectedToRequester,
      );
      expect(distributionAfter.honestNodeAmount).to.equal(
        distributionBefore.honestNodeAmount + expectedToHonestNodes,
      );
      expect(distributionAfter.totalSlashed).to.equal(slashedAmount);
    });

    it("queues slashed funds arriving before processE3Failure and applies on calculate", async function () {
      const {
        enclave,
        e3RefundManager,
        makeRequest,
        owner,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest();

      // Fail E3 but DON'T call processE3Failure yet
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(0);

      const slashedAmount = ethers.parseUnits("50", 6);

      // Escrow slashed funds BEFORE processE3Failure — should be queued
      const originalEnclave = await e3RefundManager.enclave();
      await e3RefundManager.setEnclave(await owner.getAddress());
      await e3RefundManager.connect(owner).escrowSlashedFunds(0, slashedAmount);
      await e3RefundManager.setEnclave(originalEnclave);

      // Distribution should not exist yet
      const distBefore = await e3RefundManager.getRefundDistribution(0);
      expect(distBefore.calculated).to.be.false;

      // Now process the failure — pending funds should be applied
      await enclave.processE3Failure(0);

      const distAfter = await e3RefundManager.getRefundDistribution(0);
      expect(distAfter.calculated).to.be.true;
      expect(distAfter.totalSlashed).to.equal(slashedAmount);

      // Invariant: all funds accounted for
      expect(
        distAfter.requesterAmount +
          distAfter.honestNodeAmount +
          distAfter.protocolAmount,
      ).to.equal(distAfter.originalPayment + slashedAmount);
    });
  });

  describe("Full Failure Flow - DKG Timeout", function () {
    it("complete flow: request -> committee formed -> DKG timeout -> fail -> process -> claim", async function () {
      const {
        enclave,
        e3RefundManager,
        registry,
        usdcToken,
        makeRequest,
        requester,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      // 1. Make request
      await makeRequest();
      let stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(1); // Requested

      // 2. Complete sortition (committee finalized, DKG starts)
      await registry.connect(operator1).submitTicket(0, 1);
      await registry.connect(operator2).submitTicket(0, 1);
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(0);

      stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(2); // CommitteeFinalized

      // 3. Fast forward past DKG deadline (key never published - simulating DKG failure)
      await time.increase(defaultTimeoutConfig.dkgWindow + 1);

      // 4. Check failure condition and mark as failed
      const [canFail, reason] = await enclave.checkFailureCondition(0);
      expect(canFail).to.be.true;
      expect(reason).to.equal(3); // DKGTimeout

      await enclave.markE3Failed(0);
      stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(6); // Failed

      const failureReason = await enclave.getFailureReason(0);
      expect(failureReason).to.equal(3); // DKGTimeout

      // 5. Process failure and claim refund
      await enclave.processE3Failure(0);

      const balanceBefore = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      await e3RefundManager.connect(requester).claimRequesterRefund(0);
      const balanceAfter = await usdcToken.balanceOf(
        await requester.getAddress(),
      );

      const distribution = await e3RefundManager.getRefundDistribution(0);
      expect(balanceAfter - balanceBefore).to.equal(
        distribution.requesterAmount,
      );
    });
  });

  describe("Full Failure Flow - Compute Timeout", function () {
    it("complete flow: request -> activated -> compute timeout -> fail -> process -> claim", async function () {
      const {
        enclave,
        e3RefundManager,
        registry,
        usdcToken,
        makeRequest,
        requester,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      // 1. Make request
      await makeRequest();
      let stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(1); // Requested

      // 2. Complete sortition and DKG
      await registry.connect(operator1).submitTicket(0, 1);
      await registry.connect(operator2).submitTicket(0, 1);
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(0);

      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
      ];
      const publicKey = "0x1234567890abcdef1234567890abcdef";
      const publicKeyHash = ethers.keccak256(publicKey);
      await registry.publishCommittee(0, nodes, publicKey, publicKeyHash);

      stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(3); // KeyPublished

      // 3. Wait past compute deadline (ciphertext never published)
      const e3 = await enclave.getE3(0);
      const computeDeadline =
        Number(e3.inputWindow[1]) + defaultTimeoutConfig.computeWindow;
      await time.increaseTo(computeDeadline + 1);

      // 4. Check failure condition and mark as failed
      const [canFail, reason] = await enclave.checkFailureCondition(0);
      expect(canFail).to.be.true;
      expect(reason).to.equal(6); // ComputeTimeout

      await enclave.markE3Failed(0);
      stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(6); // Failed

      const failureReason = await enclave.getFailureReason(0);
      expect(failureReason).to.equal(6); // ComputeTimeout

      // 5. Process and claim
      await enclave.processE3Failure(0);

      const balanceBefore = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      await e3RefundManager.connect(requester).claimRequesterRefund(0);
      const balanceAfter = await usdcToken.balanceOf(
        await requester.getAddress(),
      );

      const distribution = await e3RefundManager.getRefundDistribution(0);
      expect(balanceAfter - balanceBefore).to.equal(
        distribution.requesterAmount,
      );
    });
  });

  describe("Full Failure Flow - Decryption Timeout", function () {
    it("complete flow: request -> ciphertext published -> decryption timeout -> fail -> process -> claim", async function () {
      const {
        enclave,
        e3RefundManager,
        registry,
        usdcToken,
        makeRequest,
        requester,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      // 1. Make request
      await makeRequest();
      let stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(1); // Requested

      // 2. Complete sortition and DKG
      await registry.connect(operator1).submitTicket(0, 1);
      await registry.connect(operator2).submitTicket(0, 1);
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(0);

      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
      ];
      const publicKey = "0x1234567890abcdef1234567890abcdef";
      const publicKeyHash = ethers.keccak256(publicKey);
      await registry.publishCommittee(0, nodes, publicKey, publicKeyHash);

      stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(3); // KeyPublished

      // 3. Publish ciphertext output
      const e3 = await enclave.getE3(0);
      await time.increaseTo(Number(e3.inputWindow[1]));

      const ciphertextOutput = "0x" + "ab".repeat(100);
      const proof = "0x1337";
      await enclave.publishCiphertextOutput(0, ciphertextOutput, proof);
      stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(4); // CiphertextReady

      // 4. Wait past decryption deadline (plaintext never published)
      await time.increase(defaultTimeoutConfig.decryptionWindow + 1);

      // 5. Check failure condition and mark as failed
      const [canFail, reason] = await enclave.checkFailureCondition(0);
      expect(canFail).to.be.true;
      expect(reason).to.equal(10); // DecryptionTimeout

      await enclave.markE3Failed(0);
      stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(6); // Failed

      const failureReason = await enclave.getFailureReason(0);
      expect(failureReason).to.equal(10); // DecryptionTimeout

      // 6. Process failure and claim refund
      await enclave.processE3Failure(0);

      const balanceBefore = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      await e3RefundManager.connect(requester).claimRequesterRefund(0);
      const balanceAfter = await usdcToken.balanceOf(
        await requester.getAddress(),
      );

      const distribution = await e3RefundManager.getRefundDistribution(0);
      expect(balanceAfter - balanceBefore).to.equal(
        distribution.requesterAmount,
      );
      expect(distribution.requesterAmount).to.be.gt(0);
    });
  });

  describe("Multiple E3 Requests Isolation", function () {
    it("tracks multiple E3s independently", async function () {
      const {
        enclave,
        usdcToken,
        requester,
        e3Program,
        decryptionVerifier,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      const enclaveAddress = await enclave.getAddress();

      // Helper to make requests
      const makeRequestN = async (n: number) => {
        const startTime = (await time.latest()) + 100;
        const requestParams = {
          threshold: [2, 2] as [number, number],
          inputWindow: [startTime, startTime + ONE_DAY] as [number, number],
          e3Program: await e3Program.getAddress(),
          e3ProgramParams: encodedE3ProgramParams,
          computeProviderParams: abiCoder.encode(
            ["address"],
            [await decryptionVerifier.getAddress()],
          ),
          customParams: abiCoder.encode(
            ["address"],
            ["0x1234567890123456789012345678901234567890"],
          ),
        };
        const fee = await enclave.getE3Quote(requestParams);
        await usdcToken.connect(requester).approve(enclaveAddress, fee);
        await enclave.connect(requester).request(requestParams);
        return n;
      };

      // Make 3 requests
      await makeRequestN(0);
      await makeRequestN(1);
      await makeRequestN(2);

      // Verify all are in Requested stage
      expect(await enclave.getE3Stage(0)).to.equal(1);
      expect(await enclave.getE3Stage(1)).to.equal(1);
      expect(await enclave.getE3Stage(2)).to.equal(1);

      // Fail E3 #0 by waiting past its deadline
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(0);

      // E3 #0 is failed, but E3 #1 and #2 are still active
      expect(await enclave.getE3Stage(0)).to.equal(6); // Failed
      expect(await enclave.getE3Stage(1)).to.equal(1); // Still Requested
      expect(await enclave.getE3Stage(2)).to.equal(1); // Still Requested

      // E3 #1 and #2 also can be failed now (their deadlines have also passed)
      const [canFail1] = await enclave.checkFailureCondition(1);
      const [canFail2] = await enclave.checkFailureCondition(2);
      expect(canFail1).to.be.true;
      expect(canFail2).to.be.true;

      // But they haven't auto-failed - must be explicitly marked
      expect(await enclave.getE3Stage(1)).to.equal(1);
      expect(await enclave.getE3Stage(2)).to.equal(1);

      // Now mark E3 #2 as failed (but not #1)
      await enclave.markE3Failed(2);
      expect(await enclave.getE3Stage(2)).to.equal(6); // Now Failed
      expect(await enclave.getE3Stage(1)).to.equal(1); // Still Requested

      // Verify each E3 has independent failure reasons
      expect(await enclave.getFailureReason(0)).to.equal(1); // CommitteeFormationTimeout
      expect(await enclave.getFailureReason(2)).to.equal(1); // CommitteeFormationTimeout
    });

    it("allows claiming refunds for each failed E3 independently", async function () {
      const {
        enclave,
        e3RefundManager,
        usdcToken,
        requester,
        e3Program,
        decryptionVerifier,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      const enclaveAddress = await enclave.getAddress();

      // Make 2 requests
      for (let i = 0; i < 2; i++) {
        const startTime = (await time.latest()) + 100;
        const requestParams = {
          threshold: [2, 2] as [number, number],
          inputWindow: [startTime, startTime + ONE_DAY] as [number, number],
          e3Program: await e3Program.getAddress(),
          e3ProgramParams: encodedE3ProgramParams,
          computeProviderParams: abiCoder.encode(
            ["address"],
            [await decryptionVerifier.getAddress()],
          ),
          customParams: abiCoder.encode(
            ["address"],
            ["0x1234567890123456789012345678901234567890"],
          ),
        };
        const fee = await enclave.getE3Quote(requestParams);
        await usdcToken.connect(requester).approve(enclaveAddress, fee);
        await enclave.connect(requester).request(requestParams);
      }

      // Fail both
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(0);
      await enclave.markE3Failed(1);

      // Process both
      await enclave.processE3Failure(0);
      await enclave.processE3Failure(1);

      // Claim both refunds independently
      const balanceBefore = await usdcToken.balanceOf(
        await requester.getAddress(),
      );

      await e3RefundManager.connect(requester).claimRequesterRefund(0);
      const balanceAfterFirst = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      expect(balanceAfterFirst).to.be.gt(balanceBefore);

      await e3RefundManager.connect(requester).claimRequesterRefund(1);
      const balanceAfterSecond = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      expect(balanceAfterSecond).to.be.gt(balanceAfterFirst);

      // Verify can't claim twice
      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "AlreadyClaimed");
    });
  });

  describe("Success Path (Complete E3)", function () {
    it("distributes escrowed slashed funds to nodes and treasury on successful completion", async function () {
      const {
        enclave,
        e3RefundManager,
        registry,
        slashingManager,
        circuitVerifier,
        usdcToken,
        makeRequest,
        operator1,
        operator2,
        treasury,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      // 1. Request E3, form committee, publish key
      await makeRequest();
      await registry.connect(operator1).submitTicket(0, 1);
      await registry.connect(operator2).submitTicket(0, 1);
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(0);

      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
      ];
      const publicKey = "0x1234567890abcdef1234567890abcdef";
      const publicKeyHash = ethers.keccak256(publicKey);
      await registry.publishCommittee(0, nodes, publicKey, publicKeyHash);

      expect(await enclave.getE3Stage(0)).to.equal(3); // KeyPublished

      // 2. Slash operator1 during active E3 (before completion)
      //    With the stage-check removed, this should escrow funds in E3RefundManager
      const refundManagerAddress = await e3RefundManager.getAddress();
      const refundBalanceBefore =
        await usdcToken.balanceOf(refundManagerAddress);

      const proof = await signAndEncodeProof(
        operator1,
        0,
        await circuitVerifier.getAddress(),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_PROOF,
        proof,
      );

      // Verify USDC moved to refund manager (escrowed)
      const refundBalanceAfter =
        await usdcToken.balanceOf(refundManagerAddress);
      const actualSlashedAmount = refundBalanceAfter - refundBalanceBefore;
      expect(actualSlashedAmount).to.be.gt(0);

      // 3. Complete the E3 successfully: publish ciphertext → publish plaintext
      const e3 = await enclave.getE3(0);
      await time.increaseTo(Number(e3.inputWindow[1]));

      const ciphertextOutput = "0x" + "ab".repeat(100);
      const proofBytes = "0x1337";
      await enclave.publishCiphertextOutput(0, ciphertextOutput, proofBytes);
      expect(await enclave.getE3Stage(0)).to.equal(4); // CiphertextReady

      // Record the E3 payment (normal rewards) before completion zeroes it
      const e3Payment = await enclave.e3Payments(0);

      // Record balances before plaintext publish (which triggers _distributeRewards)
      const treasuryAddress = await treasury.getAddress();
      const treasuryBalanceBefore = await usdcToken.balanceOf(treasuryAddress);
      const op1BalanceBefore = await usdcToken.balanceOf(
        await operator1.getAddress(),
      );
      const op2BalanceBefore = await usdcToken.balanceOf(
        await operator2.getAddress(),
      );

      const plaintextOutput = "0x" + "cd".repeat(100);
      await enclave.publishPlaintextOutput(0, plaintextOutput, proofBytes);
      expect(await enclave.getE3Stage(0)).to.equal(5); // Complete

      // 4. Verify escrowed slashed funds were distributed
      //    50% to honest nodes (split equally), 50% to treasury
      const expectedSlashedToNodes =
        (actualSlashedAmount * BigInt(5000)) / BigInt(10000);
      const expectedSlashedToTreasury =
        actualSlashedAmount - expectedSlashedToNodes;

      const treasuryBalanceAfter = await usdcToken.balanceOf(treasuryAddress);

      // Treasury receives only the slashed-funds protocol share on success path
      // (normal E3 rewards go entirely to nodes via bondingRegistry.distributeRewards)
      expect(treasuryBalanceAfter - treasuryBalanceBefore).to.equal(
        expectedSlashedToTreasury,
      );

      // Honest nodes receive: normal E3 rewards (via bondingRegistry.distributeRewards)
      // + slashed-funds node share (via distributeSlashedFundsOnSuccess).
      // Both transfer directly to node addresses.
      const op1BalanceAfter = await usdcToken.balanceOf(
        await operator1.getAddress(),
      );
      const op2BalanceAfter = await usdcToken.balanceOf(
        await operator2.getAddress(),
      );
      const nodesReceivedTotal =
        op1BalanceAfter -
        op1BalanceBefore +
        (op2BalanceAfter - op2BalanceBefore);
      expect(nodesReceivedTotal).to.equal(e3Payment + expectedSlashedToNodes);

      // Verify refund manager escrowed balance was drained
      const refundBalanceFinal =
        await usdcToken.balanceOf(refundManagerAddress);
      expect(refundBalanceFinal).to.be.lt(refundBalanceAfter);
    });

    it("transitions through all stages to completion", async function () {
      const {
        enclave,
        registry,
        makeRequest,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      // 1. Make request
      await makeRequest();
      expect(await enclave.getE3Stage(0)).to.equal(1); // Requested

      // 2. Complete sortition and publish committee (CommitteeFinalized -> KeyPublished)
      await registry.connect(operator1).submitTicket(0, 1);
      await registry.connect(operator2).submitTicket(0, 1);
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(0);

      expect(await enclave.getE3Stage(0)).to.equal(2); // CommitteeFinalized

      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
      ];
      const publicKey = "0x1234567890abcdef1234567890abcdef";
      const publicKeyHash = ethers.keccak256(publicKey);
      await registry.publishCommittee(0, nodes, publicKey, publicKeyHash);

      expect(await enclave.getE3Stage(0)).to.equal(3); // KeyPublished

      // 3. Publish ciphertext output (after input deadline)
      const e3 = await enclave.getE3(0);
      await time.increaseTo(Number(e3.inputWindow[1]));

      const ciphertextOutput = "0x" + "ab".repeat(100);
      const proof = "0x1337";
      await enclave.publishCiphertextOutput(0, ciphertextOutput, proof);
      expect(await enclave.getE3Stage(0)).to.equal(4); // CiphertextReady

      // 4. Publish plaintext output
      const plaintextOutput = "0x" + "cd".repeat(100);
      await enclave.publishPlaintextOutput(0, plaintextOutput, proof);
      expect(await enclave.getE3Stage(0)).to.equal(5); // Complete

      // Cannot mark completed E3 as failed
      await expect(enclave.markE3Failed(0)).to.be.revertedWithCustomError(
        enclave,
        "E3AlreadyComplete",
      );
    });

    it("prevents refund claims for completed E3", async function () {
      const {
        enclave,
        e3RefundManager,
        registry,
        makeRequest,
        requester,
        operator1,
        operator2,
        setupOperator,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      // Complete full E3 flow
      await makeRequest();

      // Complete sortition
      await registry.connect(operator1).submitTicket(0, 1);
      await registry.connect(operator2).submitTicket(0, 1);
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(0);

      const nodes = [
        await operator1.getAddress(),
        await operator2.getAddress(),
      ];
      const publicKey = "0x1234567890abcdef1234567890abcdef";
      const publicKeyHash = ethers.keccak256(publicKey);
      await registry.publishCommittee(0, nodes, publicKey, publicKeyHash);

      // Publish outputs
      const e3 = await enclave.getE3(0);
      await time.increaseTo(Number(e3.inputWindow[1]));

      const ciphertextOutput = "0x" + "ab".repeat(100);
      const proof = "0x1337";
      await enclave.publishCiphertextOutput(0, ciphertextOutput, proof);

      const plaintextOutput = "0x" + "cd".repeat(100);
      await enclave.publishPlaintextOutput(0, plaintextOutput, proof);

      // Verify E3 is complete
      expect(await enclave.getE3Stage(0)).to.equal(5); // Complete

      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "RefundNotCalculated");
    });
  });
});
