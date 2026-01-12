// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import type { Signer } from "ethers";
import { network } from "hardhat";

import BondingRegistryModule from "../../ignition/modules/bondingRegistry";
import E3LifecycleModule from "../../ignition/modules/e3Lifecycle";
import E3RefundManagerModule from "../../ignition/modules/e3RefundManager";
import EnclaveTicketTokenModule from "../../ignition/modules/enclaveTicketToken";
import EnclaveTokenModule from "../../ignition/modules/enclaveToken";
import MockStableTokenModule from "../../ignition/modules/mockStableToken";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  E3Lifecycle__factory as E3LifecycleFactory,
  E3RefundManager__factory as E3RefundManagerFactory,
  MockUSDC__factory as MockUSDCFactory,
} from "../../types";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture, time, mine } = networkHelpers;

describe("E3RefundManager", function () {
  // Time constants in seconds
  const ONE_HOUR = 60 * 60;
  const ONE_DAY = 24 * ONE_HOUR;
  const THREE_DAYS = 3 * ONE_DAY;
  const SEVEN_DAYS = 7 * ONE_DAY;

  // Default timeout configuration
  const defaultTimeoutConfig = {
    committeeFormationWindow: ONE_DAY,
    dkgWindow: ONE_DAY,
    computeWindow: THREE_DAYS,
    decryptionWindow: ONE_DAY,
    gracePeriod: ONE_HOUR,
  };

  // Work allocation in basis points (10000 = 100%) 
  const defaultWorkAllocation = {
    committeeFormationBps: 1000,
    dkgBps: 3000,
    decryptionBps: 5500,
    protocolBps: 500,
  };

  const PAYMENT_AMOUNT = ethers.parseUnits("100", 6); // 100 USDC

  const setup = async () => {
    const [
      owner,
      notTheOwner,
      enclave,
      requester,
      treasury,
      honestNode1,
      honestNode2,
      faultyNode,
    ] = await ethers.getSigners();

    const ownerAddress = await owner.getAddress();
    const enclaveAddress = await enclave.getAddress();
    const treasuryAddress = await treasury.getAddress();

    // Deploy USDC mock
    const usdcTokenContract = await ignition.deploy(MockStableTokenModule, {
      parameters: {
        MockUSDC: {
          initialSupply: 1000000,
        },
      },
    });
    const usdcToken = MockUSDCFactory.connect(
      await usdcTokenContract.mockUSDC.getAddress(),
      owner,
    );

    // Deploy ENCL token for bonding
    const enclTokenContract = await ignition.deploy(EnclaveTokenModule, {
      parameters: {
        EnclaveToken: {
          owner: ownerAddress,
        },
      },
    });

    // Deploy ticket token
    const ticketTokenContract = await ignition.deploy(
      EnclaveTicketTokenModule,
      {
        parameters: {
          EnclaveTicketToken: {
            baseToken: await usdcToken.getAddress(),
            registry: ownerAddress, // temporary, will be updated
            owner: ownerAddress,
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
            licenseToken: await enclTokenContract.enclaveToken.getAddress(),
            registry: ownerAddress, // temporary
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

    // Setup: Set refund manager as reward distributor on bonding registry
    await bondingRegistry.setRewardDistributor(e3RefundManagerAddress);

    // Mint USDC to requester and refund manager for testing
    await usdcToken.mint(
      await requester.getAddress(),
      ethers.parseUnits("10000", 6),
    );
    await usdcToken.mint(e3RefundManagerAddress, ethers.parseUnits("10000", 6));
    await usdcToken.mint(treasuryAddress, ethers.parseUnits("10000", 6));

    // Helper function to initialize and fail an E3
    const initializeAndFailE3 = async (
      e3Id: number,
      failureReason: number,
    ): Promise<void> => {
      const requesterAddress = await requester.getAddress();

      // Initialize E3
      await e3Lifecycle.connect(enclave).initializeE3(e3Id, requesterAddress);

      if (failureReason === 1) {
        // CommitteeFormationTimeout
        await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
        await e3Lifecycle.markE3Failed(e3Id);
        return;
      }

      // Progress to CommitteeFinalized (DKG starts)
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(e3Id);

      if (failureReason === 3) {
        // DKGTimeout - committee finalized but key never published
        await time.increase(defaultTimeoutConfig.dkgWindow + 1);
        await e3Lifecycle.markE3Failed(e3Id);
        return;
      }

      // Progress to KeyPublished (DKG complete)
      const activationDeadline = (await time.latest()) + SEVEN_DAYS;
      await e3Lifecycle
        .connect(enclave)
        .onKeyPublished(e3Id, activationDeadline);

      // Progress to Activated
      const expiration = (await time.latest()) + ONE_DAY;
      await e3Lifecycle.connect(enclave).onActivated(e3Id, expiration);

      if (failureReason === 7) {
        // ComputeTimeout
        await time.increase(ONE_DAY + defaultTimeoutConfig.computeWindow + 1);
        await e3Lifecycle.markE3Failed(e3Id);
        return;
      }

      // Progress to CiphertextReady
      await e3Lifecycle.connect(enclave).onCiphertextPublished(e3Id);

      if (failureReason === 11) {
        // DecryptionTimeout
        await time.increase(defaultTimeoutConfig.decryptionWindow + 1);
        await e3Lifecycle.markE3Failed(e3Id);
        return;
      }
    };

    return {
      e3Lifecycle,
      e3RefundManager,
      bondingRegistry,
      usdcToken,
      owner,
      notTheOwner,
      enclave,
      requester,
      treasury,
      honestNode1,
      honestNode2,
      faultyNode,
      initializeAndFailE3,
    };
  };

  describe("initialize()", function () {
    it("correctly sets owner", async function () {
      const { e3RefundManager, owner } = await loadFixture(setup);
      expect(await e3RefundManager.owner()).to.equal(await owner.getAddress());
    });

    it("correctly sets enclave address", async function () {
      const { e3RefundManager, enclave } = await loadFixture(setup);
      expect(await e3RefundManager.enclave()).to.equal(
        await enclave.getAddress(),
      );
    });

    it("correctly sets e3Lifecycle address", async function () {
      const { e3RefundManager, e3Lifecycle } = await loadFixture(setup);
      expect(await e3RefundManager.e3Lifecycle()).to.equal(
        await e3Lifecycle.getAddress(),
      );
    });

    it("correctly sets fee token", async function () {
      const { e3RefundManager, usdcToken } = await loadFixture(setup);
      expect(await e3RefundManager.feeToken()).to.equal(
        await usdcToken.getAddress(),
      );
    });

    it("correctly sets treasury", async function () {
      const { e3RefundManager, treasury } = await loadFixture(setup);
      expect(await e3RefundManager.treasury()).to.equal(
        await treasury.getAddress(),
      );
    });

    it("correctly sets default work allocation", async function () {
      const { e3RefundManager } = await loadFixture(setup);
      const allocation = await e3RefundManager.getWorkAllocation();

      expect(allocation.committeeFormationBps).to.equal(
        defaultWorkAllocation.committeeFormationBps,
      );
      expect(allocation.dkgBps).to.equal(defaultWorkAllocation.dkgBps);
      expect(allocation.decryptionBps).to.equal(
        defaultWorkAllocation.decryptionBps,
      );
      expect(allocation.protocolBps).to.equal(
        defaultWorkAllocation.protocolBps,
      );
    });
  });

  describe("calculateRefund()", function () {
    it("reverts if not called by enclave", async function () {
      const { e3RefundManager, initializeAndFailE3, notTheOwner, honestNode1 } =
        await loadFixture(setup);

      await initializeAndFailE3(0, 1); // CommitteeFormationTimeout

      await expect(
        e3RefundManager
          .connect(notTheOwner)
          .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]),
      ).to.be.revertedWithCustomError(e3RefundManager, "Unauthorized");
    });

    it("reverts if E3 is not failed", async function () {
      const { e3RefundManager, e3Lifecycle, enclave, requester, honestNode1 } =
        await loadFixture(setup);

      // Initialize E3 but don't fail it
      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      await expect(
        e3RefundManager
          .connect(enclave)
          .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]),
      ).to.be.revertedWithCustomError(e3RefundManager, "E3NotFailed");
    });

    it("calculates refund correctly for committee formation timeout", async function () {
      const {
        e3RefundManager,
        enclave,
        initializeAndFailE3,
        honestNode1,
        honestNode2,
      } = await loadFixture(setup);

      await initializeAndFailE3(0, 1); // CommitteeFormationTimeout

      const honestNodes = [
        await honestNode1.getAddress(),
        await honestNode2.getAddress(),
      ];

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, honestNodes);

      const distribution = await e3RefundManager.getRefundDistribution(0);

      expect(distribution.calculated).to.be.true;
      expect(distribution.honestNodeCount).to.equal(2);
      expect(distribution.requesterAmount).to.equal(
        (PAYMENT_AMOUNT * 9500n) / 10000n,
      );
      expect(distribution.honestNodeAmount).to.equal(0n);
    });

    it("calculates refund correctly for DKG timeout", async function () {
      const { e3RefundManager, enclave, initializeAndFailE3, honestNode1 } =
        await loadFixture(setup);

      await initializeAndFailE3(0, 3); // DKGTimeout

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      const distribution = await e3RefundManager.getRefundDistribution(0);

      expect(distribution.honestNodeAmount).to.equal(
        (PAYMENT_AMOUNT * 1000n) / 10000n,
      );
      expect(distribution.requesterAmount).to.equal(
        (PAYMENT_AMOUNT * 8500n) / 10000n,
      );
    });

    it("calculates refund correctly for decryption timeout", async function () {
      const { e3RefundManager, enclave, initializeAndFailE3, honestNode1 } =
        await loadFixture(setup);

      await initializeAndFailE3(0, 11); // DecryptionTimeout

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      const distribution = await e3RefundManager.getRefundDistribution(0);

      expect(distribution.honestNodeAmount).to.equal(
        (PAYMENT_AMOUNT * 4000n) / 10000n,
      );
      expect(distribution.requesterAmount).to.equal(
        (PAYMENT_AMOUNT * 5500n) / 10000n,
      );
    });

    it("emits RefundDistributionCalculated event", async function () {
      const { e3RefundManager, enclave, initializeAndFailE3, honestNode1 } =
        await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      await expect(
        e3RefundManager
          .connect(enclave)
          .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]),
      ).to.emit(e3RefundManager, "RefundDistributionCalculated");
    });

    it("reverts if already calculated", async function () {
      const { e3RefundManager, enclave, initializeAndFailE3, honestNode1 } =
        await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      const honestNodes = [await honestNode1.getAddress()];

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, honestNodes);

      await expect(
        e3RefundManager
          .connect(enclave)
          .calculateRefund(0, PAYMENT_AMOUNT, honestNodes),
      ).to.be.revertedWith("Already calculated");
    });
  });

  describe("claimRequesterRefund()", function () {
    it("allows requester to claim refund", async function () {
      const {
        e3RefundManager,
        e3Lifecycle,
        enclave,
        requester,
        usdcToken,
        honestNode1,
        initializeAndFailE3,
      } = await loadFixture(setup);

      await initializeAndFailE3(0, 1); // CommitteeFormationTimeout

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      const distribution = await e3RefundManager.getRefundDistribution(0);
      const balanceBefore = await usdcToken.balanceOf(
        await requester.getAddress(),
      );

      await e3RefundManager.connect(requester).claimRequesterRefund(0);

      const balanceAfter = await usdcToken.balanceOf(
        await requester.getAddress(),
      );
      expect(balanceAfter - balanceBefore).to.equal(
        distribution.requesterAmount,
      );
    });

    it("emits RefundClaimed event", async function () {
      const {
        e3RefundManager,
        enclave,
        requester,
        honestNode1,
        initializeAndFailE3,
      } = await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.emit(e3RefundManager, "RefundClaimed");
    });

    it("reverts if E3 not failed", async function () {
      const { e3RefundManager, e3Lifecycle, enclave, requester } =
        await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "E3NotFailed");
    });

    it("reverts if refund not calculated", async function () {
      const { e3RefundManager, requester, initializeAndFailE3 } =
        await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "RefundNotCalculated");
    });

    it("reverts if not the requester", async function () {
      const {
        e3RefundManager,
        enclave,
        notTheOwner,
        honestNode1,
        initializeAndFailE3,
      } = await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      await expect(
        e3RefundManager.connect(notTheOwner).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "NotRequester");
    });

    it("reverts if already claimed", async function () {
      const {
        e3RefundManager,
        enclave,
        requester,
        honestNode1,
        initializeAndFailE3,
      } = await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      await e3RefundManager.connect(requester).claimRequesterRefund(0);

      await expect(
        e3RefundManager.connect(requester).claimRequesterRefund(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "AlreadyClaimed");
    });
  });

  describe("claimHonestNodeReward()", function () {
    it("allows honest node to claim reward", async function () {
      const {
        e3RefundManager,
        enclave,
        honestNode1,
        honestNode2,
        initializeAndFailE3,
      } = await loadFixture(setup);

      // Use DKG timeout so nodes have done some work
      await initializeAndFailE3(0, 3);

      const honestNodes = [
        await honestNode1.getAddress(),
        await honestNode2.getAddress(),
      ];

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, honestNodes);

      const distribution = await e3RefundManager.getRefundDistribution(0);
      const expectedAmount =
        distribution.honestNodeAmount / BigInt(honestNodes.length);

      // Note: The actual transfer goes through BondingRegistry.distributeRewards
      // which has its own logic. This test just verifies the claim succeeds.
      await expect(
        e3RefundManager.connect(honestNode1).claimHonestNodeReward(0),
      ).to.emit(e3RefundManager, "RefundClaimed");
    });

    it("reverts if not an honest node", async function () {
      const {
        e3RefundManager,
        enclave,
        honestNode1,
        faultyNode,
        initializeAndFailE3,
      } = await loadFixture(setup);

      await initializeAndFailE3(0, 3);

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      await expect(
        e3RefundManager.connect(faultyNode).claimHonestNodeReward(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "NotHonestNode");
    });

    it("reverts if already claimed", async function () {
      const { e3RefundManager, enclave, honestNode1, initializeAndFailE3 } =
        await loadFixture(setup);

      await initializeAndFailE3(0, 3);

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      await e3RefundManager.connect(honestNode1).claimHonestNodeReward(0);

      await expect(
        e3RefundManager.connect(honestNode1).claimHonestNodeReward(0),
      ).to.be.revertedWithCustomError(e3RefundManager, "AlreadyClaimed");
    });
  });

  describe("routeSlashedFunds()", function () {
    it("reverts if not called by enclave", async function () {
      const {
        e3RefundManager,
        notTheOwner,
        enclave,
        honestNode1,
        initializeAndFailE3,
      } = await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      const slashedAmount = ethers.parseUnits("10", 6);

      await expect(
        e3RefundManager
          .connect(notTheOwner)
          .routeSlashedFunds(0, slashedAmount),
      ).to.be.revertedWithCustomError(e3RefundManager, "Unauthorized");
    });

    it("adds slashed funds to distribution", async function () {
      const { e3RefundManager, enclave, honestNode1, initializeAndFailE3 } =
        await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      const distributionBefore = await e3RefundManager.getRefundDistribution(0);
      const slashedAmount = ethers.parseUnits("10", 6);

      await e3RefundManager
        .connect(enclave)
        .routeSlashedFunds(0, slashedAmount);

      const distributionAfter = await e3RefundManager.getRefundDistribution(0);

      expect(distributionAfter.requesterAmount).to.equal(
        distributionBefore.requesterAmount + slashedAmount / 2n,
      );
      expect(distributionAfter.honestNodeAmount).to.equal(
        distributionBefore.honestNodeAmount + slashedAmount / 2n,
      );
      expect(distributionAfter.totalSlashed).to.equal(slashedAmount);
    });

    it("emits SlashedFundsRouted event", async function () {
      const { e3RefundManager, enclave, honestNode1, initializeAndFailE3 } =
        await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      const slashedAmount = ethers.parseUnits("10", 6);

      await expect(
        e3RefundManager.connect(enclave).routeSlashedFunds(0, slashedAmount),
      )
        .to.emit(e3RefundManager, "SlashedFundsRouted")
        .withArgs(0, slashedAmount);
    });
  });

  describe("calculateWorkValue()", function () {
    it("returns 0% for None/Requested stage", async function () {
      const { e3RefundManager } = await loadFixture(setup);

      const [workCompleted, workRemaining] =
        await e3RefundManager.calculateWorkValue(0);
      expect(workCompleted).to.equal(0);
      expect(workRemaining).to.equal(9500);

      const [workCompleted2, workRemaining2] =
        await e3RefundManager.calculateWorkValue(1);
      expect(workCompleted2).to.equal(0);
    });

    it("returns 10% for CommitteeFinalized stage", async function () {
      const { e3RefundManager } = await loadFixture(setup);

      const [workCompleted, workRemaining] =
        await e3RefundManager.calculateWorkValue(2);
      expect(workCompleted).to.equal(1000);
      expect(workRemaining).to.equal(8500);
    });

    it("returns 40% for KeyPublished stage", async function () {
      const { e3RefundManager } = await loadFixture(setup);

      const [workCompleted, workRemaining] =
        await e3RefundManager.calculateWorkValue(3);
      expect(workCompleted).to.equal(4000);
      expect(workRemaining).to.equal(5500);
    });

    it("returns 40% for Activated stage", async function () {
      const { e3RefundManager } = await loadFixture(setup);

      const [workCompleted, workRemaining] =
        await e3RefundManager.calculateWorkValue(4);
      expect(workCompleted).to.equal(4000);
      expect(workRemaining).to.equal(5500);
    });

    it("returns 40% for CiphertextReady stage", async function () {
      const { e3RefundManager } = await loadFixture(setup);

      const [workCompleted, workRemaining] =
        await e3RefundManager.calculateWorkValue(5);
      expect(workCompleted).to.equal(4000);
      expect(workRemaining).to.equal(5500);
    });
  });

  describe("setWorkAllocation()", function () {
    it("reverts if not called by owner", async function () {
      const { e3RefundManager, notTheOwner } = await loadFixture(setup);

      await expect(
        e3RefundManager.connect(notTheOwner).setWorkAllocation({
          ...defaultWorkAllocation,
          committeeFormationBps: 1000,
        }),
      ).to.be.revertedWithCustomError(
        e3RefundManager,
        "OwnableUnauthorizedAccount",
      );
    });

    it("updates work allocation", async function () {
      const { e3RefundManager } = await loadFixture(setup);

      const newAllocation = {
        committeeFormationBps: 1500,
        dkgBps: 2500,
        decryptionBps: 5500,
        protocolBps: 500,
      };

      await e3RefundManager.setWorkAllocation(newAllocation);

      const allocation = await e3RefundManager.getWorkAllocation();
      expect(allocation.committeeFormationBps).to.equal(1500);
      expect(allocation.dkgBps).to.equal(2500);
      expect(allocation.decryptionBps).to.equal(5500);
    });

    it("emits WorkAllocationUpdated event", async function () {
      const { e3RefundManager } = await loadFixture(setup);

      const newAllocation = {
        committeeFormationBps: 1500,
        dkgBps: 2500,
        decryptionBps: 5500,
        protocolBps: 500,
      };

      await expect(e3RefundManager.setWorkAllocation(newAllocation)).to.emit(
        e3RefundManager,
        "WorkAllocationUpdated",
      );
    });

    it("reverts if allocation does not sum to 10000", async function () {
      const { e3RefundManager } = await loadFixture(setup);

      const invalidAllocation = {
        committeeFormationBps: 1000,
        dkgBps: 1000,
        decryptionBps: 1000,
        protocolBps: 1000, // Total: 4000, not 10000
      };

      await expect(
        e3RefundManager.setWorkAllocation(invalidAllocation),
      ).to.be.revertedWith("Must sum to 10000");
    });
  });

  describe("hasClaimed()", function () {
    it("returns false before claiming", async function () {
      const {
        e3RefundManager,
        enclave,
        requester,
        honestNode1,
        initializeAndFailE3,
      } = await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      const hasClaimed = await e3RefundManager.hasClaimed(
        0,
        await requester.getAddress(),
      );
      expect(hasClaimed).to.be.false;
    });

    it("returns true after claiming", async function () {
      const {
        e3RefundManager,
        enclave,
        requester,
        honestNode1,
        initializeAndFailE3,
      } = await loadFixture(setup);

      await initializeAndFailE3(0, 1);

      await e3RefundManager
        .connect(enclave)
        .calculateRefund(0, PAYMENT_AMOUNT, [await honestNode1.getAddress()]);

      await e3RefundManager.connect(requester).claimRequesterRefund(0);

      const hasClaimed = await e3RefundManager.hasClaimed(
        0,
        await requester.getAddress(),
      );
      expect(hasClaimed).to.be.true;
    });
  });
});
