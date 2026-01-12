// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import type { Signer } from "ethers";
import { network } from "hardhat";

import E3LifecycleModule from "../../ignition/modules/e3Lifecycle";
import MockStableTokenModule from "../../ignition/modules/mockStableToken";
import {
  E3Lifecycle__factory as E3LifecycleFactory,
  MockUSDC__factory as MockUSDCFactory,
} from "../../types";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture, time, mine } = networkHelpers;

describe("E3Lifecycle", function () {
  // Time constants in seconds
  const ONE_HOUR = 60 * 60;
  const ONE_DAY = 24 * ONE_HOUR;
  const THREE_DAYS = 3 * ONE_DAY;
  const SEVEN_DAYS = 7 * ONE_DAY;

  // Default activation deadline offset (used in tests)
  const DEFAULT_ACTIVATION_DEADLINE_OFFSET = SEVEN_DAYS;

  // Default timeout configuration
  const defaultTimeoutConfig = {
    committeeFormationWindow: ONE_DAY,
    dkgWindow: ONE_DAY,
    computeWindow: THREE_DAYS,
    decryptionWindow: ONE_DAY,
    gracePeriod: ONE_HOUR,
  };

  const setup = async () => {
    const [owner, notTheOwner, enclave, requester, operator1, operator2] =
      await ethers.getSigners();
    const ownerAddress = await owner.getAddress();
    const enclaveAddress = await enclave.getAddress();

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

    return {
      e3Lifecycle,
      owner,
      notTheOwner,
      enclave,
      requester,
      operator1,
      operator2,
    };
  };

  describe("initialize()", function () {
    it("correctly sets owner", async function () {
      const { e3Lifecycle, owner } = await loadFixture(setup);
      expect(await e3Lifecycle.owner()).to.equal(await owner.getAddress());
    });

    it("correctly sets enclave address", async function () {
      const { e3Lifecycle, enclave } = await loadFixture(setup);
      expect(await e3Lifecycle.enclave()).to.equal(await enclave.getAddress());
    });

    it("correctly sets timeout config", async function () {
      const { e3Lifecycle } = await loadFixture(setup);
      const config = await e3Lifecycle.getTimeoutConfig();

      expect(config.committeeFormationWindow).to.equal(
        defaultTimeoutConfig.committeeFormationWindow,
      );
      expect(config.dkgWindow).to.equal(defaultTimeoutConfig.dkgWindow);
      expect(config.computeWindow).to.equal(defaultTimeoutConfig.computeWindow);
      expect(config.decryptionWindow).to.equal(
        defaultTimeoutConfig.decryptionWindow,
      );
      expect(config.gracePeriod).to.equal(defaultTimeoutConfig.gracePeriod);
    });
  });

  describe("initializeE3()", function () {
    it("reverts if not called by enclave", async function () {
      const { e3Lifecycle, notTheOwner, requester } = await loadFixture(setup);

      await expect(
        e3Lifecycle
          .connect(notTheOwner)
          .initializeE3(0, await requester.getAddress()),
      ).to.be.revertedWithCustomError(e3Lifecycle, "Unauthorized");
    });

    it("sets E3 stage to Requested", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(1); // E3Stage.Requested = 1
    });

    it("sets requester correctly", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);
      const requesterAddress = await requester.getAddress();

      await e3Lifecycle.connect(enclave).initializeE3(0, requesterAddress);

      expect(await e3Lifecycle.getRequester(0)).to.equal(requesterAddress);
    });

    it("sets committee deadline correctly", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      const tx = await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      const block = await ethers.provider.getBlock(tx.blockNumber!);

      const deadlines = await e3Lifecycle.getDeadlines(0);
      expect(deadlines.committeeDeadline).to.equal(
        block!.timestamp + defaultTimeoutConfig.committeeFormationWindow,
      );
    });

    it("emits E3StageChanged event", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await expect(
        e3Lifecycle
          .connect(enclave)
          .initializeE3(0, await requester.getAddress()),
      )
        .to.emit(e3Lifecycle, "E3StageChanged")
        .withArgs(0, 0, 1); // None -> Requested
    });

    it("reverts if E3 already exists", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);
      const requesterAddress = await requester.getAddress();

      await e3Lifecycle.connect(enclave).initializeE3(0, requesterAddress);

      await expect(
        e3Lifecycle.connect(enclave).initializeE3(0, requesterAddress),
      ).to.be.revertedWith("E3 already exists");
    });
  });

  describe("onCommitteeFinalized()", function () {
    it("reverts if not called by enclave", async function () {
      const { e3Lifecycle, notTheOwner, enclave, requester } =
        await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      await expect(
        e3Lifecycle.connect(notTheOwner).onCommitteeFinalized(0),
      ).to.be.revertedWithCustomError(e3Lifecycle, "Unauthorized");
    });

    it("transitions from Requested to CommitteeFinalized", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(2); // E3Stage.CommitteeFinalized = 2
    });

    it("sets DKG deadline correctly", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      const tx = await e3Lifecycle
        .connect(enclave)
        .onCommitteeFinalized(0);
      const block = await ethers.provider.getBlock(tx.blockNumber!);

      const deadlines = await e3Lifecycle.getDeadlines(0);
      expect(deadlines.dkgDeadline).to.equal(
        block!.timestamp + defaultTimeoutConfig.dkgWindow,
      );
    });

    it("emits E3StageChanged event", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      await expect(
        e3Lifecycle.connect(enclave).onCommitteeFinalized(0),
      )
        .to.emit(e3Lifecycle, "E3StageChanged")
        .withArgs(0, 1, 2); // Requested -> CommitteeFinalized
    });

    it("reverts if not in Requested stage", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      // E3 doesn't exist - stage is None
      await expect(
        e3Lifecycle.connect(enclave).onCommitteeFinalized(0),
      ).to.be.revertedWithCustomError(e3Lifecycle, "InvalidStage");
    });
  });

  describe("onKeyPublished()", function () {
    it("reverts if not called by enclave", async function () {
      const { e3Lifecycle, notTheOwner, enclave, requester } =
        await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;

      await expect(
        e3Lifecycle.connect(notTheOwner).onKeyPublished(0, activationDeadline),
      ).to.be.revertedWithCustomError(e3Lifecycle, "Unauthorized");
    });

    it("transitions from CommitteeFinalized to KeyPublished", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(3); // E3Stage.KeyPublished = 3
    });

    it("sets activation deadline correctly", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);

      const deadlines = await e3Lifecycle.getDeadlines(0);
      expect(deadlines.activationDeadline).to.equal(activationDeadline);
    });

    it("emits E3StageChanged event", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;

      await expect(
        e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline),
      )
        .to.emit(e3Lifecycle, "E3StageChanged")
        .withArgs(0, 2, 3); // CommitteeFinalized -> KeyPublished
    });

    it("reverts if not in CommitteeFinalized stage", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;

      // E3 is in Requested stage, not CommitteeFinalized
      await expect(
        e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline),
      ).to.be.revertedWithCustomError(e3Lifecycle, "InvalidStage");
    });
  });

  describe("onActivated()", function () {
    const inputDeadline = Math.floor(Date.now() / 1000) + ONE_DAY;

    it("reverts if not called by enclave", async function () {
      const { e3Lifecycle, notTheOwner, enclave, requester } =
        await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);

      await expect(
        e3Lifecycle.connect(notTheOwner).onActivated(0, inputDeadline),
      ).to.be.revertedWithCustomError(e3Lifecycle, "Unauthorized");
    });

    it("transitions from KeyPublished to Activated", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);
      await e3Lifecycle.connect(enclave).onActivated(0, inputDeadline);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(4); // E3Stage.Activated = 4
    });

    it("sets compute deadline correctly", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);
      await e3Lifecycle.connect(enclave).onActivated(0, inputDeadline);

      const deadlines = await e3Lifecycle.getDeadlines(0);
      expect(deadlines.computeDeadline).to.equal(
        inputDeadline + defaultTimeoutConfig.computeWindow,
      );
    });

    it("emits E3StageChanged event", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);

      await expect(e3Lifecycle.connect(enclave).onActivated(0, inputDeadline))
        .to.emit(e3Lifecycle, "E3StageChanged")
        .withArgs(0, 3, 4); // KeyPublished -> Activated
    });
  });

  describe("onCiphertextPublished()", function () {
    it("sets decryption deadline correctly", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);
      const inputDeadline = (await time.latest()) + ONE_DAY;

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);
      await e3Lifecycle.connect(enclave).onActivated(0, inputDeadline);
      const tx = await e3Lifecycle.connect(enclave).onCiphertextPublished(0);
      const block = await ethers.provider.getBlock(tx.blockNumber!);

      const deadlines = await e3Lifecycle.getDeadlines(0);
      expect(deadlines.decryptionDeadline).to.equal(
        block!.timestamp + defaultTimeoutConfig.decryptionWindow,
      );
    });

    it("transitions to CiphertextReady stage", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);
      const inputDeadline = (await time.latest()) + ONE_DAY;

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);
      await e3Lifecycle.connect(enclave).onActivated(0, inputDeadline);
      await e3Lifecycle.connect(enclave).onCiphertextPublished(0);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(5); // E3Stage.CiphertextReady = 5
    });
  });

  describe("onComplete()", function () {
    it("transitions to Complete stage", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);
      const inputDeadline = (await time.latest()) + ONE_DAY;

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);
      await e3Lifecycle.connect(enclave).onActivated(0, inputDeadline);
      await e3Lifecycle.connect(enclave).onCiphertextPublished(0);
      await e3Lifecycle.connect(enclave).onComplete(0);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(6); // E3Stage.Complete = 6
    });

    it("emits E3StageChanged event", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);
      const inputDeadline = (await time.latest()) + ONE_DAY;

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);
      await e3Lifecycle.connect(enclave).onActivated(0, inputDeadline);
      await e3Lifecycle.connect(enclave).onCiphertextPublished(0);

      await expect(e3Lifecycle.connect(enclave).onComplete(0))
        .to.emit(e3Lifecycle, "E3StageChanged")
        .withArgs(0, 5, 6); // CiphertextReady -> Complete
    });
  });

  describe("markE3Failed()", function () {
    it("marks E3 as failed when committee formation times out", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      // Fast forward past committee deadline
      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);

      await e3Lifecycle.markE3Failed(0);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // E3Stage.Failed = 7

      const reason = await e3Lifecycle.getFailureReason(0);
      expect(reason).to.equal(1); // FailureReason.CommitteeFormationTimeout = 1
    });

    it("marks E3 as failed when DKG times out", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);

      // Fast forward past DKG deadline
      await time.increase(defaultTimeoutConfig.dkgWindow + 1);

      await e3Lifecycle.markE3Failed(0);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // E3Stage.Failed = 7

      const reason = await e3Lifecycle.getFailureReason(0);
      expect(reason).to.equal(3); // FailureReason.DKGTimeout = 3
    });

    it("marks E3 as failed when compute times out", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);
      const inputDeadline = (await time.latest()) + ONE_DAY;

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);
      await e3Lifecycle.connect(enclave).onActivated(0, inputDeadline);

      // Fast forward past compute deadline
      await time.increase(
        ONE_DAY + defaultTimeoutConfig.computeWindow + 1,
      );

      await e3Lifecycle.markE3Failed(0);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // E3Stage.Failed = 7

      const reason = await e3Lifecycle.getFailureReason(0);
      expect(reason).to.equal(7); // FailureReason.ComputeTimeout = 7
    });

    it("marks E3 as failed when decryption times out", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);
      const inputDeadline = (await time.latest()) + ONE_DAY;

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);
      await e3Lifecycle.connect(enclave).onActivated(0, inputDeadline);
      await e3Lifecycle.connect(enclave).onCiphertextPublished(0);

      // Fast forward past decryption deadline
      await time.increase(defaultTimeoutConfig.decryptionWindow + 1);

      await e3Lifecycle.markE3Failed(0);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // E3Stage.Failed = 7

      const reason = await e3Lifecycle.getFailureReason(0);
      expect(reason).to.equal(11); // FailureReason.DecryptionTimeout = 11
    });

    it("marks E3 as failed when activation window expires", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);

      // Set activation deadline to be 1 hour in the future
      const activationDeadline = (await time.latest()) + ONE_HOUR;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);

      // E3 is now in KeyPublished stage, but we don't activate it
      // Fast forward past activation deadline
      await time.increase(ONE_HOUR + 1);

      // Should be able to mark as failed due to activation window expiry
      await e3Lifecycle.markE3Failed(0);

      const stage = await e3Lifecycle.getE3Stage(0);
      expect(stage).to.equal(7); // E3Stage.Failed = 7

      const reason = await e3Lifecycle.getFailureReason(0);
      expect(reason).to.equal(5); // FailureReason.ActivationWindowExpired = 5
    });

    it("emits E3Failed event", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);

      await expect(e3Lifecycle.markE3Failed(0))
        .to.emit(e3Lifecycle, "E3Failed")
        .withArgs(0, 1, 1); // e3Id, failedAtStage (Requested), reason (CommitteeFormationTimeout)
    });

    it("reverts if E3 does not exist", async function () {
      const { e3Lifecycle } = await loadFixture(setup);

      await expect(
        e3Lifecycle.markE3Failed(99),
      ).to.be.revertedWithCustomError(e3Lifecycle, "InvalidStage");
    });

    it("reverts if E3 is already complete", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);
      const inputDeadline = (await time.latest()) + ONE_DAY;

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());
      await e3Lifecycle.connect(enclave).onCommitteeFinalized(0);
      const activationDeadline =
        (await time.latest()) + DEFAULT_ACTIVATION_DEADLINE_OFFSET;
      await e3Lifecycle.connect(enclave).onKeyPublished(0, activationDeadline);
      await e3Lifecycle.connect(enclave).onActivated(0, inputDeadline);
      await e3Lifecycle.connect(enclave).onCiphertextPublished(0);
      await e3Lifecycle.connect(enclave).onComplete(0);

      await expect(
        e3Lifecycle.markE3Failed(0),
      ).to.be.revertedWithCustomError(e3Lifecycle, "E3AlreadyComplete");
    });

    it("reverts if E3 is already failed", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);
      await e3Lifecycle.markE3Failed(0);

      await expect(
        e3Lifecycle.markE3Failed(0),
      ).to.be.revertedWithCustomError(e3Lifecycle, "E3AlreadyFailed");
    });

    it("reverts if failure condition not met", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      // Don't advance time - deadline not passed
      await expect(
        e3Lifecycle.markE3Failed(0),
      ).to.be.revertedWithCustomError(e3Lifecycle, "FailureConditionNotMet");
    });
  });

  describe("checkFailureCondition()", function () {
    it("returns false when no timeout has occurred", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      const [canFail, reason] = await e3Lifecycle.checkFailureCondition(0);
      expect(canFail).to.be.false;
      expect(reason).to.equal(0); // FailureReason.None
    });

    it("returns true when committee formation times out", async function () {
      const { e3Lifecycle, enclave, requester } = await loadFixture(setup);

      await e3Lifecycle
        .connect(enclave)
        .initializeE3(0, await requester.getAddress());

      await time.increase(defaultTimeoutConfig.committeeFormationWindow + 1);

      const [canFail, reason] = await e3Lifecycle.checkFailureCondition(0);
      expect(canFail).to.be.true;
      expect(reason).to.equal(1); // FailureReason.CommitteeFormationTimeout
    });
  });

  describe("setTimeoutConfig()", function () {
    it("reverts if not called by owner", async function () {
      const { e3Lifecycle, notTheOwner } = await loadFixture(setup);

      await expect(
        e3Lifecycle.connect(notTheOwner).setTimeoutConfig({
          ...defaultTimeoutConfig,
          committeeFormationWindow: ONE_HOUR,
        }),
      ).to.be.revertedWithCustomError(e3Lifecycle, "OwnableUnauthorizedAccount");
    });

    it("updates timeout config", async function () {
      const { e3Lifecycle } = await loadFixture(setup);

      const newConfig = {
        committeeFormationWindow: 2 * ONE_DAY,
        dkgWindow: 2 * ONE_DAY,
        computeWindow: SEVEN_DAYS,
        decryptionWindow: 2 * ONE_DAY,
        gracePeriod: 2 * ONE_HOUR,
      };

      await e3Lifecycle.setTimeoutConfig(newConfig);

      const config = await e3Lifecycle.getTimeoutConfig();
      expect(config.committeeFormationWindow).to.equal(
        newConfig.committeeFormationWindow,
      );
      expect(config.dkgWindow).to.equal(newConfig.dkgWindow);
      expect(config.computeWindow).to.equal(newConfig.computeWindow);
      expect(config.decryptionWindow).to.equal(newConfig.decryptionWindow);
      expect(config.gracePeriod).to.equal(newConfig.gracePeriod);
    });

    it("emits TimeoutConfigUpdated event", async function () {
      const { e3Lifecycle } = await loadFixture(setup);

      const newConfig = {
        committeeFormationWindow: 2 * ONE_DAY,
        dkgWindow: 2 * ONE_DAY,
        computeWindow: SEVEN_DAYS,
        decryptionWindow: 2 * ONE_DAY,
        gracePeriod: 2 * ONE_HOUR,
      };

      await expect(e3Lifecycle.setTimeoutConfig(newConfig))
        .to.emit(e3Lifecycle, "TimeoutConfigUpdated");
    });

    it("reverts if any window is zero", async function () {
      const { e3Lifecycle } = await loadFixture(setup);

      await expect(
        e3Lifecycle.setTimeoutConfig({
          ...defaultTimeoutConfig,
          committeeFormationWindow: 0,
        }),
      ).to.be.revertedWith("Invalid committee window");

      await expect(
        e3Lifecycle.setTimeoutConfig({
          ...defaultTimeoutConfig,
          dkgWindow: 0,
        }),
      ).to.be.revertedWith("Invalid DKG window");

      await expect(
        e3Lifecycle.setTimeoutConfig({
          ...defaultTimeoutConfig,
          computeWindow: 0,
        }),
      ).to.be.revertedWith("Invalid compute window");

      await expect(
        e3Lifecycle.setTimeoutConfig({
          ...defaultTimeoutConfig,
          decryptionWindow: 0,
        }),
      ).to.be.revertedWith("Invalid decryption window");
    });
  });

  describe("setEnclave()", function () {
    it("reverts if not called by owner", async function () {
      const { e3Lifecycle, notTheOwner, operator1 } = await loadFixture(setup);

      await expect(
        e3Lifecycle
          .connect(notTheOwner)
          .setEnclave(await operator1.getAddress()),
      ).to.be.revertedWithCustomError(e3Lifecycle, "OwnableUnauthorizedAccount");
    });

    it("updates enclave address", async function () {
      const { e3Lifecycle, operator1 } = await loadFixture(setup);
      const newEnclaveAddress = await operator1.getAddress();

      await e3Lifecycle.setEnclave(newEnclaveAddress);

      expect(await e3Lifecycle.enclave()).to.equal(newEnclaveAddress);
    });

    it("reverts if address is zero", async function () {
      const { e3Lifecycle } = await loadFixture(setup);

      await expect(
        e3Lifecycle.setEnclave(ethers.ZeroAddress),
      ).to.be.revertedWith("Invalid enclave address");
    });
  });
});
