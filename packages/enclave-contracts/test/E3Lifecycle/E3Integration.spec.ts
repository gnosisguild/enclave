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
import MockStableTokenModule from "../../ignition/modules/mockStableToken";
import SlashingManagerModule from "../../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryOwnableFactory,
  E3RefundManager__factory as E3RefundManagerFactory,
  Enclave__factory as EnclaveFactory,
  EnclaveToken__factory as EnclaveTokenFactory,
  MockDecryptionVerifier__factory as MockDecryptionVerifierFactory,
  MockE3Program__factory as MockE3ProgramFactory,
  MockUSDC__factory as MockUSDCFactory,
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
    gracePeriod: ONE_HOUR,
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

  const setup = async () => {
    const [owner, requester, treasury, operator1, operator2, computeProvider] =
      await ethers.getSigners();

    const ownerAddress = await owner.getAddress();
    const treasuryAddress = await treasury.getAddress();
    const requesterAddress = await requester.getAddress();

    // Deploy USDC mock
    const usdcContract = await ignition.deploy(MockStableTokenModule, {
      parameters: {
        MockUSDC: {
          initialSupply: 10000000,
        },
      },
    });
    const usdcToken = MockUSDCFactory.connect(
      await usdcContract.mockUSDC.getAddress(),
      owner,
    );

    // Deploy ENCL token
    const enclTokenContract = await ignition.deploy(EnclaveTokenModule, {
      parameters: {
        EnclaveToken: {
          owner: ownerAddress,
        },
      },
    });
    const enclToken = EnclaveTokenFactory.connect(
      await enclTokenContract.enclaveToken.getAddress(),
      owner,
    );

    // Deploy ticket token
    const ticketTokenContract = await ignition.deploy(
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

    // Deploy slashing manager
    const slashingManagerContract = await ignition.deploy(
      SlashingManagerModule,
      {
        parameters: {
          SlashingManager: {
            admin: ownerAddress,
            bondingRegistry: addressOne, // Will be updated
          },
        },
      },
    );

    // Deploy bonding registry
    const bondingRegistryContract = await ignition.deploy(
      BondingRegistryModule,
      {
        parameters: {
          BondingRegistry: {
            owner: ownerAddress,
            ticketToken:
              await ticketTokenContract.enclaveTicketToken.getAddress(),
            licenseToken: await enclToken.getAddress(),
            registry: addressOne, // Will be updated
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
      await bondingRegistryContract.bondingRegistry.getAddress(),
      owner,
    );

    // Deploy Enclave (with addressOne as temp registry)
    const enclaveContract = await ignition.deploy(EnclaveModule, {
      parameters: {
        Enclave: {
          params: encodedE3ProgramParams,
          owner: ownerAddress,
          maxDuration: THIRTY_DAYS,
          registry: addressOne,
          e3RefundManager: addressOne,
          bondingRegistry: await bondingRegistry.getAddress(),
          feeToken: await usdcToken.getAddress(),
          timeoutConfig: defaultTimeoutConfig,
        },
      },
    });
    const enclaveAddress = await enclaveContract.enclave.getAddress();
    const enclave = EnclaveFactory.connect(enclaveAddress, owner);

    // Deploy CiphernodeRegistry
    const ciphernodeRegistry = await ignition.deploy(CiphernodeRegistryModule, {
      parameters: {
        CiphernodeRegistry: {
          enclaveAddress: enclaveAddress,
          owner: ownerAddress,
          submissionWindow: SORTITION_SUBMISSION_WINDOW,
        },
      },
    });
    const ciphernodeRegistryAddress =
      await ciphernodeRegistry.cipherNodeRegistry.getAddress();
    const registry = CiphernodeRegistryOwnableFactory.connect(
      ciphernodeRegistryAddress,
      owner,
    );

    // Deploy E3RefundManager
    const e3RefundManagerContract = await ignition.deploy(
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
    const e3RefundManagerAddress =
      await e3RefundManagerContract.e3RefundManager.getAddress();
    const e3RefundManager = E3RefundManagerFactory.connect(
      e3RefundManagerAddress,
      owner,
    );

    // Deploy mock E3 Program
    const e3ProgramContract = await ignition.deploy(MockE3ProgramModule, {
      parameters: {
        MockE3Program: {
          encryptionSchemeId: encryptionSchemeId,
        },
      },
    });
    const e3Program = MockE3ProgramFactory.connect(
      await e3ProgramContract.mockE3Program.getAddress(),
      owner,
    );

    // Deploy mock decryption verifier
    const decryptionVerifierContract = await ignition.deploy(
      MockDecryptionVerifierModule,
    );
    const decryptionVerifier = MockDecryptionVerifierFactory.connect(
      await decryptionVerifierContract.mockDecryptionVerifier.getAddress(),
      owner,
    );

    // Wire up all the contracts
    await enclave.setCiphernodeRegistry(ciphernodeRegistryAddress);
    await enclave.setE3RefundManager(e3RefundManagerAddress);
    await enclave.enableE3Program(await e3Program.getAddress());
    await enclave.setDecryptionVerifier(
      encryptionSchemeId,
      await decryptionVerifier.getAddress(),
    );

    // Setup bonding registry connections
    await bondingRegistry.setRewardDistributor(enclaveAddress);
    await bondingRegistry.setRegistry(ciphernodeRegistryAddress);
    await bondingRegistry.setSlashingManager(
      await slashingManagerContract.slashingManager.getAddress(),
    );
    await slashingManagerContract.slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );
    await registry.setBondingRegistry(await bondingRegistry.getAddress());

    // Update ticket token registry
    await ticketTokenContract.enclaveTicketToken.setRegistry(
      await bondingRegistry.getAddress(),
    );

    // Mint tokens to requester
    await usdcToken.mint(requesterAddress, ethers.parseUnits("10000", 6));
    // Mint tokens to refund manager for distribution tests
    await usdcToken.mint(e3RefundManagerAddress, ethers.parseUnits("10000", 6));

    // Helper to make E3 request
    const makeRequest = async (
      signer: Signer = requester,
    ): Promise<{ e3Id: number }> => {
      const startTime = (await time.latest()) + 100;

      const requestParams = {
        threshold: [2, 2] as [number, number],
        inputWindow: [startTime + 100, startTime + ONE_DAY] as [number, number],
        e3Program: await e3Program.getAddress(),
        e3ProgramParams: encodedE3ProgramParams,
        // computeProviderParams must be exactly 32 bytes for MockE3Program.validate
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

      // Get e3Id from event (it's 0 for first request)
      return { e3Id: 0 };
    };

    async function setupOperator(operator: Signer) {
      const operatorAddress = await operator.getAddress();

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

      const ticketTokenAddress = await bondingRegistry.ticketToken();
      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator)
        .approve(ticketTokenAddress, ticketAmount);
      await bondingRegistry.connect(operator).addTicketBalance(ticketAmount);
    }

    return {
      enclave,
      e3RefundManager,
      bondingRegistry,
      registry,
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

  describe("E3RefundManager Initialization", function () {
    it("correctly sets enclave address", async function () {
      const { enclave, e3RefundManager } = await loadFixture(setup);

      expect(await e3RefundManager.enclave()).to.equal(
        await enclave.getAddress(),
      );
    });
  });

  describe("Full Failure Flow - Committee Formation Timeout", function () {
    it("complete flow: request -> timeout -> fail -> process -> claim", async function () {
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

      // 1. Make request
      await makeRequest();

      // Verify stage
      let stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(1); // Requested

      // 2. Fast forward past deadline
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);

      // 3. Anyone can mark as failed
      const [canFail, reason] = await enclave.checkFailureCondition(0);
      expect(canFail).to.be.true;
      expect(reason).to.equal(1); // CommitteeFormationTimeout

      await enclave.markE3Failed(0);
      stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(6); // Failed

      // 4. Process failure
      await enclave.processE3Failure(0);

      // 5. Requester claims refund
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

  describe("Slashed Funds Routing", function () {
    it("routes slashed funds 50/50 to requester and honest nodes", async function () {
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

      // Fail the E3
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await enclave.markE3Failed(0);
      await enclave.processE3Failure(0);

      const distributionBefore = await e3RefundManager.getRefundDistribution(0);
      const slashedAmount = ethers.parseUnits("100", 6);

      // Route slashed funds (normally called by SlashingManager through Enclave)
      // For testing, temporarily set enclave to owner to call this permissioned function
      const originalEnclave = await e3RefundManager.enclave();
      await e3RefundManager.setEnclave(await owner.getAddress());
      await e3RefundManager.connect(owner).routeSlashedFunds(0, slashedAmount);
      await e3RefundManager.setEnclave(originalEnclave);

      const distributionAfter = await e3RefundManager.getRefundDistribution(0);

      // Verify slashed funds are split 50/50 between requester and honest nodes
      expect(distributionAfter.requesterAmount).to.equal(
        distributionBefore.requesterAmount + slashedAmount / 2n,
      );
      expect(distributionAfter.honestNodeAmount).to.equal(
        distributionBefore.honestNodeAmount + slashedAmount / 2n,
      );
      expect(distributionAfter.totalSlashed).to.equal(slashedAmount);
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
