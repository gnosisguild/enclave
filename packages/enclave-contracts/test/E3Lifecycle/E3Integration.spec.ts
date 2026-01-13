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
import E3LifecycleModule from "../../ignition/modules/e3Lifecycle";
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
  E3Lifecycle__factory as E3LifecycleFactory,
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
 * - Enclave.sol (main coordinator)
 * - E3Lifecycle.sol (stage tracking and timeout detection)
 * - E3RefundManager.sol (refund calculation and claiming)
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
          e3Lifecycle: addressOne,
          e3RefundManager: addressOne,
          bondingRegistry: await bondingRegistry.getAddress(),
          feeToken: await usdcToken.getAddress(),
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

    // Deploy E3Lifecycle
    const e3LifecycleContract = await ignition.deploy(E3LifecycleModule, {
      parameters: {
        E3Lifecycle: {
          owner: ownerAddress,
          enclave: enclaveAddress,
          ...defaultTimeoutConfig,
        },
      },
    });
    const e3LifecycleAddress =
      await e3LifecycleContract.e3Lifecycle.getAddress();
    const e3Lifecycle = E3LifecycleFactory.connect(e3LifecycleAddress, owner);

    // Deploy E3RefundManager
    const e3RefundManagerContract = await ignition.deploy(
      E3RefundManagerModule,
      {
        parameters: {
          E3RefundManager: {
            owner: ownerAddress,
            enclave: enclaveAddress,
            e3Lifecycle: e3LifecycleAddress,
            feeToken: await usdcToken.getAddress(),
            bondingRegistry: await bondingRegistry.getAddress(),
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
    await enclave.setE3Lifecycle(e3LifecycleAddress);
    await enclave.setE3RefundManager(e3RefundManagerAddress);
    await enclave.enableE3Program(await e3Program.getAddress());
    await enclave.setDecryptionVerifier(
      encryptionSchemeId,
      await decryptionVerifier.getAddress(),
    );

    // Setup bonding registry connections
    await bondingRegistry.setRewardDistributor(e3RefundManagerAddress);
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
        startWindow: [startTime, startTime + ONE_DAY] as [number, number],
        duration: ONE_DAY,
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

    return {
      enclave,
      e3Lifecycle,
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
    };
  };

  describe("E3 Request with Lifecycle Integration", function () {
    it("initializes E3 lifecycle when request is made", async function () {
      const { e3Lifecycle, makeRequest, requester } = await loadFixture(setup);

      await makeRequest();

      // Check that E3 lifecycle was initialized
      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(1); // E3Stage.Requested

      // Check requester is tracked
      const storedRequester = await e3Lifecycle.getRequester(0);
      expect(storedRequester).to.equal(await requester.getAddress());
    });

    it("sets committee formation deadline on request", async function () {
      const { e3Lifecycle, makeRequest } = await loadFixture(setup);

      const beforeTime = await time.latest();
      await makeRequest();
      const afterTime = await time.latest();

      const deadlines = await e3Lifecycle.getDeadlines(0);
      expect(deadlines.committeeDeadline).to.be.gte(
        beforeTime + defaultTimeoutConfig.committeeFormationWindow,
      );
      expect(deadlines.committeeDeadline).to.be.lte(
        afterTime + defaultTimeoutConfig.committeeFormationWindow + 1,
      );
    });
  });

  describe("Committee Formed Integration", function () {
    // Helper to setup an operator for sortition
    async function setupOperatorForSortition(
      operator: Signer,
      bondingRegistry: any,
      enclToken: any,
      usdcToken: any,
      _registry: any,
      _owner: Signer,
    ): Promise<void> {
      const operatorAddress = await operator.getAddress();

      // Enable token transfers
      await enclToken.setTransferRestriction(false);

      // Mint license tokens to operator
      await enclToken.mintAllocation(
        operatorAddress,
        ethers.parseEther("10000"),
        "Test allocation",
      );

      // Mint USDC to operator
      await usdcToken.mint(operatorAddress, ethers.parseUnits("100000", 6));

      // Approve and bond license
      await enclToken
        .connect(operator)
        .approve(await bondingRegistry.getAddress(), ethers.parseEther("2000"));
      await bondingRegistry
        .connect(operator)
        .bondLicense(ethers.parseEther("1000"));
      await bondingRegistry.connect(operator).registerOperator();

      // Get ticket token address from bonding registry and add ticket balance
      const ticketTokenAddress = await bondingRegistry.ticketToken();
      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator)
        .approve(ticketTokenAddress, ticketAmount);
      await bondingRegistry.connect(operator).addTicketBalance(ticketAmount);

      // Note: addCiphernode is called internally by registerOperator via bondingRegistry
    }

    it("transitions to CommitteeFormed when publishCommittee is called", async function () {
      const {
        e3Lifecycle,
        registry,
        bondingRegistry,
        usdcToken,
        enclToken,
        makeRequest,
        owner,
        operator1,
        operator2,
      } = await loadFixture(setup);

      // Setup operators for sortition
      await setupOperatorForSortition(
        operator1,
        bondingRegistry,
        enclToken,
        usdcToken,
        registry,
        owner,
      );
      await setupOperatorForSortition(
        operator2,
        bondingRegistry,
        enclToken,
        usdcToken,
        registry,
        owner,
      );

      // Make a request first
      await makeRequest();

      // Verify stage is Requested
      let stage = await e3Lifecycle.getE3Stage(0);
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
      stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(3); // E3Stage.KeyPublished

      // Verify deadlines were set
      const deadlines = await e3Lifecycle.getDeadlines(0);
      expect(deadlines.dkgDeadline).to.be.gt(0);
      expect(deadlines.activationDeadline).to.be.gt(0);
    });

    it("emits CommitteeFormed event when committee is published", async function () {
      const {
        enclave,
        registry,
        bondingRegistry,
        usdcToken,
        enclToken,
        makeRequest,
        owner,
        operator1,
        operator2,
      } = await loadFixture(setup);

      // Setup operators for sortition
      await setupOperatorForSortition(
        operator1,
        bondingRegistry,
        enclToken,
        usdcToken,
        registry,
        owner,
      );
      await setupOperatorForSortition(
        operator2,
        bondingRegistry,
        enclToken,
        usdcToken,
        registry,
        owner,
      );

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
      const { enclave, owner, makeRequest } = await loadFixture(setup);

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
            e3Lifecycle: addressOne,
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
      const { enclave, makeRequest } = await loadFixture(setup);

      await makeRequest();

      // E3 is in Requested state, not Failed
      await expect(enclave.processE3Failure(0)).to.be.revertedWith(
        "E3 not failed",
      );
    });

    it("processes failure and calculates refund for committee formation timeout", async function () {
      const { enclave, e3Lifecycle, e3RefundManager, makeRequest } =
        await loadFixture(setup);

      await makeRequest();

      // Fast forward past committee formation deadline
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);

      // Mark E3 as failed
      await e3Lifecycle.markE3Failed(0);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // E3Stage.Failed

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
        e3Lifecycle,
        e3RefundManager,
        makeRequest,
        requester,
        usdcToken,
      } = await loadFixture(setup);

      await makeRequest();

      // Get initial balance
      const balanceBefore = await usdcToken.balanceOf(
        await requester.getAddress(),
      );

      // Fast forward and fail E3
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await e3Lifecycle.markE3Failed(0);
      await enclave.processE3Failure(0);

      // Claim refund
      await e3RefundManager.connect(requester).claimRequesterRefund(0);

      const balanceAfter = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      expect(balanceAfter).to.be.gt(balanceBefore);
    });

    it("reverts if trying to process failure twice", async function () {
      const { enclave, e3Lifecycle, makeRequest } = await loadFixture(setup);

      await makeRequest();

      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await e3Lifecycle.markE3Failed(0);
      await enclave.processE3Failure(0);

      // Second call should fail - payment already cleared
      await expect(enclave.processE3Failure(0)).to.be.revertedWith(
        "No payment to refund",
      );
    });
  });

  describe("Full Failure Flow - Committee Formation Timeout", function () {
    it("complete flow: request -> timeout -> fail -> process -> claim", async function () {
      const {
        enclave,
        e3Lifecycle,
        e3RefundManager,
        makeRequest,
        requester,
        usdcToken,
      } = await loadFixture(setup);

      // 1. Make request
      await makeRequest();

      // Verify stage
      let stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(1); // Requested

      // 2. Fast forward past deadline
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);

      // 3. Anyone can mark as failed
      const [canFail, reason] = await e3Lifecycle.checkFailureCondition(0);
      expect(canFail).to.be.true;
      expect(reason).to.equal(1); // CommitteeFormationTimeout

      await e3Lifecycle.markE3Failed(0);
      stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // Failed

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
      const { enclave, e3Lifecycle, e3RefundManager, makeRequest } =
        await loadFixture(setup);

      await makeRequest();

      // Fail the E3
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await e3Lifecycle.markE3Failed(0);
      await enclave.processE3Failure(0);

      const distributionBefore = await e3RefundManager.getRefundDistribution(0);
      const slashedAmount = ethers.parseUnits("100", 6);

      // Route slashed funds (normally called by SlashingManager integration)
      // We test via enclave's permission
      await e3RefundManager.setEnclave(await enclave.owner());
      await e3RefundManager.routeSlashedFunds(0, slashedAmount);

      const distributionAfter = await e3RefundManager.getRefundDistribution(0);

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
        e3Lifecycle,
        e3RefundManager,
        makeRequest,
        requester,
        usdcToken,
        owner,
      } = await loadFixture(setup);

      // 1. Make request
      await makeRequest();
      let stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(1); // Requested

      // 2. Simulate committee finalized (DKG starts but will timeout)
      // For DKG timeout, we only call onCommitteeFinalized - key is never published
      await e3Lifecycle.setEnclave(await owner.getAddress());
      await e3Lifecycle.connect(owner).onCommitteeFinalized(0);
      await e3Lifecycle.setEnclave(await enclave.getAddress());

      stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(2); // CommitteeFinalized

      // 3. Fast forward past DKG deadline (key never published)
      await time.increase(defaultTimeoutConfig.dkgWindow + 1);

      // 4. Check failure condition and mark as failed
      const [canFail, reason] = await e3Lifecycle.checkFailureCondition(0);
      expect(canFail).to.be.true;
      expect(reason).to.equal(3); // DKGTimeout

      await e3Lifecycle.markE3Failed(0);
      stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // Failed

      const failureReason = await e3Lifecycle.getFailureReason(0);
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

  describe("Full Failure Flow - Activation Window Expiry", function () {
    it("complete flow: request -> committee formed -> activation expires -> fail -> process -> claim", async function () {
      const {
        enclave,
        e3Lifecycle,
        e3RefundManager,
        makeRequest,
        requester,
        usdcToken,
        owner,
      } = await loadFixture(setup);

      // 1. Make request
      await makeRequest();

      // 2. Form committee with short activation deadline
      const activationDeadline = (await time.latest()) + ONE_HOUR; // Only 1 hour to activate
      await e3Lifecycle.setEnclave(await owner.getAddress());
      await e3Lifecycle.connect(owner).onCommitteeFinalized(0);
      await e3Lifecycle.connect(owner).onKeyPublished(0, activationDeadline);
      await e3Lifecycle.setEnclave(await enclave.getAddress());

      let stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(3); // KeyPublished

      // 3. Fast forward past activation deadline (but not DKG deadline)
      await time.increase(ONE_HOUR + 1);

      // 4. Check failure condition
      const [canFail, reason] = await e3Lifecycle.checkFailureCondition(0);
      expect(canFail).to.be.true;
      expect(reason).to.equal(5); // ActivationWindowExpired

      // 5. Mark as failed
      await e3Lifecycle.markE3Failed(0);
      stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // Failed

      const failureReason = await e3Lifecycle.getFailureReason(0);
      expect(failureReason).to.equal(5); // ActivationWindowExpired

      // 6. Process and claim
      await enclave.processE3Failure(0);

      const balanceBefore = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      await e3RefundManager.connect(requester).claimRequesterRefund(0);
      const balanceAfter = await usdcToken.balanceOf(
        await requester.getAddress(),
      );

      expect(balanceAfter).to.be.gt(balanceBefore);
    });
  });

  describe("Full Failure Flow - Compute Timeout", function () {
    it("complete flow: request -> activated -> compute timeout -> fail -> process -> claim", async function () {
      const {
        enclave,
        e3Lifecycle,
        e3RefundManager,
        makeRequest,
        requester,
        usdcToken,
        owner,
      } = await loadFixture(setup);

      // 1. Make request
      await makeRequest();

      // 2. Form committee
      const activationDeadline = (await time.latest()) + SEVEN_DAYS;
      await e3Lifecycle.setEnclave(await owner.getAddress());
      await e3Lifecycle.connect(owner).onCommitteeFinalized(0);
      await e3Lifecycle.connect(owner).onKeyPublished(0, activationDeadline);

      // 3. Activate (with input deadline in the future)
      const inputDeadline = (await time.latest()) + ONE_DAY;
      await e3Lifecycle.connect(owner).onActivated(0, inputDeadline);
      await e3Lifecycle.setEnclave(await enclave.getAddress());

      let stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(4); // Activated

      // 4. Fast forward past compute deadline
      // computeDeadline = inputDeadline + computeWindow
      await time.increase(ONE_DAY + defaultTimeoutConfig.computeWindow + 1);

      // 5. Check failure condition and mark as failed
      const [canFail, reason] = await e3Lifecycle.checkFailureCondition(0);
      expect(canFail).to.be.true;
      expect(reason).to.equal(7); // ComputeTimeout

      await e3Lifecycle.markE3Failed(0);
      stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // Failed

      const failureReason = await e3Lifecycle.getFailureReason(0);
      expect(failureReason).to.equal(7); // ComputeTimeout

      // 6. Process and claim
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
        e3Lifecycle,
        e3RefundManager,
        makeRequest,
        requester,
        usdcToken,
        owner,
      } = await loadFixture(setup);

      // 1. Make request
      await makeRequest();

      // 2. Advance through all stages
      const activationDeadline = (await time.latest()) + SEVEN_DAYS;
      await e3Lifecycle.setEnclave(await owner.getAddress());
      await e3Lifecycle.connect(owner).onCommitteeFinalized(0);
      await e3Lifecycle.connect(owner).onKeyPublished(0, activationDeadline);

      const inputDeadline = (await time.latest()) + ONE_DAY;
      await e3Lifecycle.connect(owner).onActivated(0, inputDeadline);
      await e3Lifecycle.connect(owner).onCiphertextPublished(0);
      await e3Lifecycle.setEnclave(await enclave.getAddress());

      let stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(5); // CiphertextReady

      // 3. Fast forward past decryption deadline
      await time.increase(defaultTimeoutConfig.decryptionWindow + 1);

      // 4. Check failure condition and mark as failed
      const [canFail, reason] = await e3Lifecycle.checkFailureCondition(0);
      expect(canFail).to.be.true;
      expect(reason).to.equal(11); // DecryptionTimeout

      await e3Lifecycle.markE3Failed(0);
      stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // Failed

      const failureReason = await e3Lifecycle.getFailureReason(0);
      expect(failureReason).to.equal(11); // DecryptionTimeout

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
      expect(distribution.requesterAmount).to.be.gt(0);
    });
  });

  describe("Multiple E3 Requests Isolation", function () {
    it("tracks multiple E3s independently", async function () {
      const {
        enclave,
        e3Lifecycle,
        usdcToken,
        requester,
        owner,
        e3Program,
        decryptionVerifier,
      } = await loadFixture(setup);

      const enclaveAddress = await enclave.getAddress();
      const abiCoder = ethers.AbiCoder.defaultAbiCoder();

      // Helper to make requests with unique IDs
      const makeRequestN = async (n: number) => {
        const startTime = (await time.latest()) + 100;
        const requestParams = {
          threshold: [2, 2] as [number, number],
          startWindow: [startTime, startTime + ONE_DAY] as [number, number],
          duration: ONE_DAY,
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
      expect(await e3Lifecycle.getE3Stage(0)).to.equal(1);
      expect(await e3Lifecycle.getE3Stage(1)).to.equal(1);
      expect(await e3Lifecycle.getE3Stage(2)).to.equal(1);

      // Advance E3 #1 to CommitteeFormed
      await e3Lifecycle.setEnclave(await owner.getAddress());
      await e3Lifecycle.connect(owner).onCommitteeFinalized(1);
      await e3Lifecycle
        .connect(owner)
        .onKeyPublished(1, (await time.latest()) + SEVEN_DAYS);
      await e3Lifecycle.setEnclave(await enclave.getAddress());

      // Verify stages are independent
      expect(await e3Lifecycle.getE3Stage(0)).to.equal(1); // Still Requested
      expect(await e3Lifecycle.getE3Stage(1)).to.equal(3); // KeyPublished
      expect(await e3Lifecycle.getE3Stage(2)).to.equal(1); // Still Requested

      // Fail E3 #0 (deadline has passed)
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await e3Lifecycle.markE3Failed(0);

      // E3 #0 is failed, E3 #1 is still KeyPublished (different deadline)
      // E3 #2 CAN be failed but hasn't been marked yet
      expect(await e3Lifecycle.getE3Stage(0)).to.equal(7); // Failed
      expect(await e3Lifecycle.getE3Stage(1)).to.equal(3); // Still KeyPublished (has activation deadline)

      // E3 #2 is still Requested until we explicitly mark it failed
      // Even though its deadline has passed, it doesn't auto-fail
      const [canFail2] = await e3Lifecycle.checkFailureCondition(2);
      expect(canFail2).to.be.true;
      expect(await e3Lifecycle.getE3Stage(2)).to.equal(1); // Still Requested (not auto-failed)

      // Now mark E3 #2 as failed
      await e3Lifecycle.markE3Failed(2);
      expect(await e3Lifecycle.getE3Stage(2)).to.equal(7); // Now Failed
    });

    it("allows claiming refunds for each failed E3 independently", async function () {
      const {
        enclave,
        e3Lifecycle,
        e3RefundManager,
        usdcToken,
        requester,
        e3Program,
        decryptionVerifier,
      } = await loadFixture(setup);

      const enclaveAddress = await enclave.getAddress();
      const abiCoder = ethers.AbiCoder.defaultAbiCoder();

      // Make 2 requests
      for (let i = 0; i < 2; i++) {
        const startTime = (await time.latest()) + 100;
        const requestParams = {
          threshold: [2, 2] as [number, number],
          startWindow: [startTime, startTime + ONE_DAY] as [number, number],
          duration: ONE_DAY,
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
      await e3Lifecycle.markE3Failed(0);
      await e3Lifecycle.markE3Failed(1);

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
      const { e3Lifecycle, makeRequest, owner, enclave } =
        await loadFixture(setup);

      await makeRequest();
      expect(await e3Lifecycle.getE3Stage(0)).to.equal(1); // Requested

      await e3Lifecycle.setEnclave(await owner.getAddress());

      await e3Lifecycle.connect(owner).onCommitteeFinalized(0);
      await e3Lifecycle
        .connect(owner)
        .onKeyPublished(0, (await time.latest()) + SEVEN_DAYS);
      expect(await e3Lifecycle.getE3Stage(0)).to.equal(3); // KeyPublished

      await e3Lifecycle
        .connect(owner)
        .onActivated(0, (await time.latest()) + ONE_DAY);
      expect(await e3Lifecycle.getE3Stage(0)).to.equal(4); // Activated

      await e3Lifecycle.connect(owner).onCiphertextPublished(0);
      expect(await e3Lifecycle.getE3Stage(0)).to.equal(5); // CiphertextReady

      await e3Lifecycle.connect(owner).onComplete(0);
      expect(await e3Lifecycle.getE3Stage(0)).to.equal(6); // Complete

      await e3Lifecycle.setEnclave(await enclave.getAddress());

      // Cannot mark completed E3 as failed
      await expect(e3Lifecycle.markE3Failed(0)).to.be.revertedWithCustomError(
        e3Lifecycle,
        "E3AlreadyComplete",
      );
    });

    it("prevents refund claims for completed E3", async function () {
      const {
        e3Lifecycle,
        e3RefundManager,
        makeRequest,
        owner,
        enclave,
        requester,
      } = await loadFixture(setup);

      await makeRequest();

      await e3Lifecycle.setEnclave(await owner.getAddress());
      await e3Lifecycle.connect(owner).onCommitteeFinalized(0);
      await e3Lifecycle
        .connect(owner)
        .onKeyPublished(0, (await time.latest()) + SEVEN_DAYS);
      await e3Lifecycle
        .connect(owner)
        .onActivated(0, (await time.latest()) + ONE_DAY);
      await e3Lifecycle.connect(owner).onCiphertextPublished(0);
      await e3Lifecycle.connect(owner).onComplete(0);
      await e3Lifecycle.setEnclave(await enclave.getAddress());

      // Refund should not be claimable for completed E3
      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "E3NotFailed");
    });
  });
});
