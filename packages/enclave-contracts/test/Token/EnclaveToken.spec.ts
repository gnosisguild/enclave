// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";

import { EnclaveToken__factory as EnclaveTokenFactory } from "../../types";
import {
  SEVEN_DAYS,
  deployEnclaveSystem,
  ethers,
  networkHelpers,
} from "../fixtures";

const { loadFixture, time } = networkHelpers;

const DAY = 24n * 60n * 60n;
const YEAR = 365n * DAY;

const GROUP_PRE_SEED = ethers.encodeBytes32String("PRE_SEED");
const GROUP_BRIDGE = ethers.encodeBytes32String("BRIDGE");
const GROUP_TEAM = ethers.encodeBytes32String("GG_TEAM");
const GROUP_CCA_REG_S = ethers.encodeBytes32String("CCA_REG_S");

describe("EnclaveToken", function () {
  async function deploy() {
    const [deployer, admin, minter, whitelister, alice, bob, claimSource] =
      await ethers.getSigners();
    const token = await new EnclaveTokenFactory(deployer).deploy(
      await admin.getAddress(),
    );
    return {
      deployer,
      admin,
      minter,
      whitelister,
      alice,
      bob,
      claimSource,
      token,
    };
  }

  async function deployWithUnlockedTransfers() {
    const fixture = await deploy();
    await fixture.token.connect(fixture.admin).setTgeEarliest(1);
    await fixture.token.connect(fixture.admin).tge();
    await fixture.token.connect(fixture.admin).disableTransferRestrictions();
    return fixture;
  }

  async function createLinearLock(
    overrides: Partial<{
      totalAmount: bigint;
      tokenHoldUntil: bigint;
      tokenUnlockStart: bigint;
      tokenUnlockEnd: bigint;
      serviceStart: bigint;
      serviceCliff: bigint;
      serviceEnd: bigint;
      group: string;
    }> = {},
  ) {
    const fixture = await loadFixture(deployWithUnlockedTransfers);
    const { token, admin, alice } = fixture;
    const aliceAddress = await alice.getAddress();
    const now = BigInt(await time.latest());
    const tge = now + DAY;
    const totalAmount = overrides.totalAmount ?? ethers.parseEther("2400");

    await token.connect(admin).setTgeTimestamp(tge);
    await token
      .connect(admin)
      .mintAllocation(aliceAddress, totalAmount, "Locked allocation");

    await token.connect(admin).createLockSchedule({
      account: aliceAddress,
      amount: totalAmount,
      tokenHoldUntil: overrides.tokenHoldUntil ?? tge,
      tokenUnlockStart: overrides.tokenUnlockStart ?? 0n,
      tokenUnlockEnd: overrides.tokenUnlockEnd ?? tge + 2n * YEAR,
      serviceStart: overrides.serviceStart ?? 0n,
      serviceCliff: overrides.serviceCliff ?? 0n,
      serviceEnd: overrides.serviceEnd ?? 0n,
      group: overrides.group ?? GROUP_PRE_SEED,
    });

    return { ...fixture, aliceAddress, tge, totalAmount };
  }

  // ── H-15 ──────────────────────────────────────────────────────────────────
  describe("H-15 — WHITELIST_ROLE separation + one-way disable", function () {
    it("admin starts with DEFAULT_ADMIN_ROLE, MINTER_ROLE, and WHITELIST_ROLE", async function () {
      const { token, admin } = await loadFixture(deploy);
      const DEFAULT_ADMIN_ROLE = await token.DEFAULT_ADMIN_ROLE();
      const MINTER_ROLE = await token.MINTER_ROLE();
      const WHITELIST_ROLE = await token.WHITELIST_ROLE();
      expect(
        await token.hasRole(DEFAULT_ADMIN_ROLE, await admin.getAddress()),
      ).to.equal(true);
      expect(
        await token.hasRole(MINTER_ROLE, await admin.getAddress()),
      ).to.equal(true);
      expect(
        await token.hasRole(WHITELIST_ROLE, await admin.getAddress()),
      ).to.equal(true);
    });

    it("non-WHITELIST_ROLE cannot call toggleTransferWhitelist", async function () {
      const { token, alice } = await loadFixture(deploy);
      await expect(
        token.connect(alice).toggleTransferWhitelist(await alice.getAddress()),
      ).to.be.revertedWithCustomError(
        token,
        "AccessControlUnauthorizedAccount",
      );
    });

    it("WHITELIST_ROLE without MINTER_ROLE can whitelist", async function () {
      const { token, admin, whitelister, alice } = await loadFixture(deploy);
      const WHITELIST_ROLE = await token.WHITELIST_ROLE();
      await token
        .connect(admin)
        .grantRole(WHITELIST_ROLE, await whitelister.getAddress());
      await expect(
        token
          .connect(whitelister)
          .toggleTransferWhitelist(await alice.getAddress()),
      )
        .to.emit(token, "TransferWhitelistUpdated")
        .withArgs(await alice.getAddress(), true);
    });

    it("non-admin cannot disableTransferRestrictions", async function () {
      const { token, alice } = await loadFixture(deploy);
      await expect(
        token.connect(alice).disableTransferRestrictions(),
      ).to.be.revertedWithCustomError(
        token,
        "AccessControlUnauthorizedAccount",
      );
    });

    it("disableTransferRestrictions is one-way (idempotent no-op on second call)", async function () {
      const { token, admin } = await loadFixture(deploy);
      await token.connect(admin).setTgeEarliest(1);
      await token.connect(admin).tge();
      await expect(token.connect(admin).disableTransferRestrictions())
        .to.emit(token, "TransferRestrictionUpdated")
        .withArgs(false);
      expect(await token.transfersRestricted()).to.equal(false);
      // Second call: idempotent no-op (does not revert, does not emit).
      await expect(
        token.connect(admin).disableTransferRestrictions(),
      ).to.not.emit(token, "TransferRestrictionUpdated");
      expect(await token.transfersRestricted()).to.equal(false);
    });
  });

  // ── M-29 ──────────────────────────────────────────────────────────────────
  describe("M-29 — EIP-6372 timestamp clock", function () {
    it("clock() returns block.timestamp and CLOCK_MODE() is mode=timestamp", async function () {
      const { token } = await loadFixture(deploy);
      expect(await token.clock()).to.equal(await time.latest());
      expect(await token.CLOCK_MODE()).to.equal("mode=timestamp");
    });
  });

  describe("pooled token-level locks", function () {
    it("requires LOCK_MANAGER_ROLE for schedule creation", async function () {
      const { token, alice } = await loadFixture(deployWithUnlockedTransfers);
      await expect(
        token.connect(alice).createLockSchedule({
          account: await alice.getAddress(),
          amount: ethers.parseEther("1"),
          tokenHoldUntil: 1n,
          tokenUnlockStart: 1n,
          tokenUnlockEnd: 1n,
          serviceStart: 0n,
          serviceCliff: 0n,
          serviceEnd: 0n,
          group: GROUP_PRE_SEED,
        }),
      ).to.be.revertedWithCustomError(
        token,
        "AccessControlUnauthorizedAccount",
      );
    });

    it("blocks transfers that would drop a wallet below its locked floor", async function () {
      const { token, alice, bob, totalAmount, tge } = await createLinearLock();
      const bobAddress = await bob.getAddress();

      expect(await token.lockedFloorOf(await alice.getAddress())).to.equal(
        totalAmount,
      );

      await expect(
        token.connect(alice).transfer(bobAddress, 1n),
      ).to.be.revertedWithCustomError(token, "LockedBalanceInvariant");

      await time.increaseTo(tge + YEAR);
      expect(await token.lockedFloorOf(await alice.getAddress())).to.equal(
        totalAmount / 2n,
      );
      expect(
        await token.transferableBalanceOf(await alice.getAddress()),
      ).to.equal(totalAmount / 2n);

      await token.connect(alice).transfer(bobAddress, totalAmount / 2n);
      await expect(
        token.connect(alice).transfer(bobAddress, totalAmount / 2n),
      ).to.be.revertedWithCustomError(token, "LockedBalanceInvariant");

      await time.increaseTo(tge + 2n * YEAR);
      await token.connect(alice).transfer(bobAddress, totalAmount / 2n);
      expect(await token.balanceOf(await alice.getAddress())).to.equal(0n);
    });

    it("keeps launch whitelist separate from locked-floor enforcement", async function () {
      const { token, admin, alice, bob } = await loadFixture(deploy);
      const aliceAddress = await alice.getAddress();
      const bobAddress = await bob.getAddress();
      const now = BigInt(await time.latest());
      const tge = now + DAY;
      const totalAmount = ethers.parseEther("100");

      await token.connect(admin).setTgeEarliest(1);
      await token.connect(admin).tge();
      await token.connect(admin).setTgeTimestamp(tge);
      await token
        .connect(admin)
        .mintAllocation(aliceAddress, totalAmount, "Locked allocation");
      await token.connect(admin).toggleTransferWhitelist(aliceAddress);
      await token.connect(admin).createLockSchedule({
        account: aliceAddress,
        amount: totalAmount,
        tokenHoldUntil: tge,
        tokenUnlockStart: 0n,
        tokenUnlockEnd: tge + YEAR,
        serviceStart: 0n,
        serviceCliff: 0n,
        serviceEnd: 0n,
        group: GROUP_PRE_SEED,
      });

      await expect(
        token.connect(alice).transfer(bobAddress, 1n),
      ).to.be.revertedWithCustomError(token, "LockedBalanceInvariant");
    });

    it("accumulates Bridge SAFT linear unlock while the hold is still active", async function () {
      const fixture = await loadFixture(deployWithUnlockedTransfers);
      const { token, admin, alice } = fixture;
      const aliceAddress = await alice.getAddress();
      const now = BigInt(await time.latest());
      const tge = now + DAY;
      const totalAmount = ethers.parseEther("2400");

      await token.connect(admin).setTgeTimestamp(tge);
      await token
        .connect(admin)
        .mintAllocation(aliceAddress, totalAmount, "Bridge allocation");
      await token.connect(admin).createLockSchedule({
        account: aliceAddress,
        amount: totalAmount,
        tokenHoldUntil: tge + YEAR,
        tokenUnlockStart: 0n,
        tokenUnlockEnd: tge + 2n * YEAR,
        serviceStart: 0n,
        serviceCliff: 0n,
        serviceEnd: 0n,
        group: GROUP_BRIDGE,
      });

      await time.increaseTo(tge + YEAR / 2n);
      expect(await token.lockedFloorOf(aliceAddress)).to.equal(totalAmount);

      await time.increaseTo(tge + YEAR);
      expect(await token.lockedFloorOf(aliceAddress)).to.equal(
        totalAmount / 2n,
      );
    });

    it("applies team service vesting as the stricter curve", async function () {
      const fixture = await loadFixture(deployWithUnlockedTransfers);
      const { token, admin, alice } = fixture;
      const aliceAddress = await alice.getAddress();
      const now = BigInt(await time.latest());
      const tge = now + DAY;
      const signing = tge - 180n * DAY;
      const totalAmount = ethers.parseEther("4800");

      await token.connect(admin).setTgeTimestamp(tge);
      await token
        .connect(admin)
        .mintAllocation(aliceAddress, totalAmount, "Team allocation");
      await token.connect(admin).createLockSchedule({
        account: aliceAddress,
        amount: totalAmount,
        tokenHoldUntil: tge,
        tokenUnlockStart: 0n,
        tokenUnlockEnd: tge + 2n * YEAR,
        serviceStart: signing,
        serviceCliff: signing + YEAR,
        serviceEnd: signing + 4n * YEAR,
        group: GROUP_TEAM,
      });

      await time.increaseTo(signing + YEAR - DAY);
      expect(await token.lockedFloorOf(aliceAddress)).to.equal(totalAmount);

      await time.increaseTo(signing + YEAR);
      expect(await token.lockedFloorOf(aliceAddress)).to.equal(
        totalAmount - totalAmount / 4n,
      );
    });

    it("creates lock schedules on transfers from approved CCA claim sources", async function () {
      const { token, admin, alice, bob, claimSource } = await loadFixture(
        deployWithUnlockedTransfers,
      );
      const aliceAddress = await alice.getAddress();
      const bobAddress = await bob.getAddress();
      const claimSourceAddress = await claimSource.getAddress();
      const claimAmount = ethers.parseEther("500");

      await token.connect(admin).setClaimSource(claimSourceAddress, true);
      await token.connect(admin).setClaimLockProfile(aliceAddress, {
        active: true,
        holdDuration: 40n * DAY,
        unlockDuration: 0n,
        group: GROUP_CCA_REG_S,
      });
      await token
        .connect(admin)
        .mintAllocation(claimSourceAddress, claimAmount, "CCA claim source");

      await expect(
        token.connect(claimSource).transfer(aliceAddress, claimAmount),
      ).to.emit(token, "LockScheduleCreated");

      expect(await token.lockedFloorOf(aliceAddress)).to.equal(claimAmount);
      await expect(
        token.connect(alice).transfer(bobAddress, 1n),
      ).to.be.revertedWithCustomError(token, "LockedBalanceInvariant");

      await time.increase(40n * DAY + 1n);
      expect(await token.lockedFloorOf(aliceAddress)).to.equal(0n);
      await token.connect(alice).transfer(bobAddress, claimAmount);
    });

    it("reuses fully unlocked schedule slots at the schedule cap", async function () {
      const { token, admin, alice } = await loadFixture(
        deployWithUnlockedTransfers,
      );
      const aliceAddress = await alice.getAddress();
      const now = BigInt(await time.latest());
      const unlockedAmount = ethers.parseEther("1");
      const activeAmount = ethers.parseEther("2");

      await token
        .connect(admin)
        .mintAllocation(
          aliceAddress,
          unlockedAmount * 64n + activeAmount,
          "Schedule capacity",
        );

      for (let i = 0; i < 64; i++) {
        await token.connect(admin).createLockSchedule({
          account: aliceAddress,
          amount: unlockedAmount,
          tokenHoldUntil: now,
          tokenUnlockStart: now,
          tokenUnlockEnd: now,
          serviceStart: 0n,
          serviceCliff: 0n,
          serviceEnd: 0n,
          group: GROUP_PRE_SEED,
        });
      }

      expect(await token.lockScheduleCount(aliceAddress)).to.equal(64n);
      expect(await token.lockedFloorOf(aliceAddress)).to.equal(0n);

      await expect(
        token.connect(admin).createLockSchedule({
          account: aliceAddress,
          amount: activeAmount,
          tokenHoldUntil: now + DAY,
          tokenUnlockStart: now + DAY,
          tokenUnlockEnd: now + 2n * DAY,
          serviceStart: 0n,
          serviceCliff: 0n,
          serviceEnd: 0n,
          group: GROUP_PRE_SEED,
        }),
      )
        .to.emit(token, "LockScheduleCreated")
        .withArgs(
          aliceAddress,
          0n,
          GROUP_PRE_SEED,
          activeAmount,
          now + DAY,
          now + DAY,
          now + 2n * DAY,
          0n,
          0n,
          0n,
        );

      expect(await token.lockScheduleCount(aliceAddress)).to.equal(64n);
      expect(await token.lockedFloorOf(aliceAddress)).to.equal(activeAmount);
    });

    it("rejects claim-source transfers when the recipient has no active profile", async function () {
      const { token, admin, alice, claimSource } = await loadFixture(
        deployWithUnlockedTransfers,
      );
      const claimSourceAddress = await claimSource.getAddress();
      const claimAmount = ethers.parseEther("1");

      await token.connect(admin).setClaimSource(claimSourceAddress, true);
      await token
        .connect(admin)
        .mintAllocation(claimSourceAddress, claimAmount, "CCA claim source");

      await expect(
        token
          .connect(claimSource)
          .transfer(await alice.getAddress(), claimAmount),
      )
        .to.be.revertedWithCustomError(token, "ClaimLockProfileMissing")
        .withArgs(await alice.getAddress());
    });

    it("does not create claim locks for BondingRegistry exit payouts", async function () {
      const signers = await ethers.getSigners();
      const [, beneficiary] = signers;
      const beneficiaryAddress = await beneficiary.getAddress();
      const sys = await deployEnclaveSystem({
        useMockCiphernodeRegistry: true,
        setupOperators: 0,
        mintUsdcTo: [],
      });
      const { bondingRegistry, licenseToken } = sys;
      const bondingRegistryAddress = await bondingRegistry.getAddress();
      const bondAmount = ethers.parseEther("100");
      const unbondAmount = ethers.parseEther("25");

      await licenseToken.setBondingRegistry(bondingRegistryAddress);
      await licenseToken.setClaimSource(bondingRegistryAddress, true);
      await licenseToken.mintAllocation(
        beneficiaryAddress,
        bondAmount,
        "License bond",
      );
      await licenseToken
        .connect(beneficiary)
        .approve(bondingRegistryAddress, bondAmount);

      await bondingRegistry.connect(beneficiary).bondLicense(bondAmount);
      await bondingRegistry.connect(beneficiary).unbondLicense(unbondAmount);

      await time.increase(SEVEN_DAYS + 1);
      await bondingRegistry.connect(beneficiary).claimExits(0, unbondAmount);

      expect(await licenseToken.lockScheduleCount(beneficiaryAddress)).to.equal(
        0n,
      );
      expect(await licenseToken.balanceOf(beneficiaryAddress)).to.equal(
        unbondAmount,
      );
    });

    it("does not let admins create schedules beyond current controlled balance", async function () {
      const { token, admin, alice } = await loadFixture(
        deployWithUnlockedTransfers,
      );
      const now = BigInt(await time.latest());
      const tge = now + DAY;

      await token.connect(admin).setTgeTimestamp(tge);
      await expect(
        token.connect(admin).createLockSchedule({
          account: await alice.getAddress(),
          amount: ethers.parseEther("1"),
          tokenHoldUntil: tge,
          tokenUnlockStart: 0n,
          tokenUnlockEnd: tge + YEAR,
          serviceStart: 0n,
          serviceCliff: 0n,
          serviceEnd: 0n,
          group: GROUP_PRE_SEED,
        }),
      ).to.be.revertedWithCustomError(token, "LockedBalanceInvariant");
    });

    it("counts self-bonded and pending-exit ENCL toward the locked floor", async function () {
      const signers = await ethers.getSigners();
      const [, beneficiary, slasher] = signers;
      const beneficiaryAddress = await beneficiary.getAddress();
      const slasherAddress = await slasher.getAddress();
      const sys = await deployEnclaveSystem({
        useMockCiphernodeRegistry: true,
        setupOperators: 0,
        wireSlashingManager: false,
        mintUsdcTo: [],
      });
      const { bondingRegistry, licenseToken, owner } = sys;
      const bondingRegistryAddress = await bondingRegistry.getAddress();
      const now = BigInt(await time.latest());
      const tge = now + DAY;
      const totalAmount = ethers.parseEther("1000");
      const bondAmount = ethers.parseEther("800");
      const unbondAmount = ethers.parseEther("300");

      await bondingRegistry.setSlashingManager(slasherAddress);
      await licenseToken.setTgeTimestamp(tge);
      await licenseToken.setBondingRegistry(bondingRegistryAddress);
      await licenseToken.mintAllocation(
        beneficiaryAddress,
        totalAmount,
        "Locked allocation",
      );
      await licenseToken.createLockSchedule({
        account: beneficiaryAddress,
        amount: totalAmount,
        tokenHoldUntil: tge,
        tokenUnlockStart: 0n,
        tokenUnlockEnd: tge + YEAR,
        serviceStart: 0n,
        serviceCliff: 0n,
        serviceEnd: 0n,
        group: GROUP_PRE_SEED,
      });

      await licenseToken
        .connect(beneficiary)
        .approve(bondingRegistryAddress, bondAmount);
      await bondingRegistry.connect(beneficiary).bondLicense(bondAmount);

      expect(await licenseToken.balanceOf(beneficiaryAddress)).to.equal(
        totalAmount - bondAmount,
      );
      expect(await licenseToken.totalBondedOf(beneficiaryAddress)).to.equal(
        bondAmount,
      );
      expect(
        await licenseToken.transferableBalanceOf(beneficiaryAddress),
      ).to.equal(0n);

      await bondingRegistry.connect(beneficiary).unbondLicense(unbondAmount);
      expect(await licenseToken.totalBondedOf(beneficiaryAddress)).to.equal(
        bondAmount,
      );

      await time.increase(SEVEN_DAYS + 1);
      await bondingRegistry.connect(beneficiary).claimExits(0, unbondAmount);
      expect(await licenseToken.totalBondedOf(beneficiaryAddress)).to.equal(
        bondAmount - unbondAmount,
      );
      expect(await licenseToken.balanceOf(beneficiaryAddress)).to.equal(
        totalAmount - bondAmount + unbondAmount,
      );

      await expect(
        licenseToken
          .connect(beneficiary)
          .transfer(await owner.getAddress(), ethers.parseEther("100")),
      ).to.be.revertedWithCustomError(licenseToken, "LockedBalanceInvariant");
    });

    it("encumbers later same-wallet tokens after a slash deficit", async function () {
      const signers = await ethers.getSigners();
      const [, beneficiary, slasher, recipient] = signers;
      const beneficiaryAddress = await beneficiary.getAddress();
      const recipientAddress = await recipient.getAddress();
      const slasherAddress = await slasher.getAddress();
      const sys = await deployEnclaveSystem({
        useMockCiphernodeRegistry: true,
        setupOperators: 0,
        wireSlashingManager: false,
        mintUsdcTo: [],
      });
      const { bondingRegistry, licenseToken } = sys;
      const bondingRegistryAddress = await bondingRegistry.getAddress();
      const now = BigInt(await time.latest());
      const tge = now + DAY;
      const totalAmount = ethers.parseEther("1000");
      const slashAmount = ethers.parseEther("400");

      await bondingRegistry.setSlashingManager(slasherAddress);
      await licenseToken.setTgeTimestamp(tge);
      await licenseToken.setBondingRegistry(bondingRegistryAddress);
      await licenseToken.mintAllocation(
        beneficiaryAddress,
        totalAmount,
        "Locked allocation",
      );
      await licenseToken.createLockSchedule({
        account: beneficiaryAddress,
        amount: totalAmount,
        tokenHoldUntil: tge,
        tokenUnlockStart: 0n,
        tokenUnlockEnd: tge + YEAR,
        serviceStart: 0n,
        serviceCliff: 0n,
        serviceEnd: 0n,
        group: GROUP_PRE_SEED,
      });

      await licenseToken
        .connect(beneficiary)
        .approve(bondingRegistryAddress, totalAmount);
      await bondingRegistry.connect(beneficiary).bondLicense(totalAmount);
      await bondingRegistry
        .connect(slasher)
        .slashLicenseBond(
          beneficiaryAddress,
          slashAmount,
          ethers.encodeBytes32String("TEST_SLASH"),
        );

      await licenseToken.mintAllocation(
        beneficiaryAddress,
        slashAmount,
        "Later unlocked top-up",
      );
      await expect(
        licenseToken.connect(beneficiary).transfer(recipientAddress, 1n),
      ).to.be.revertedWithCustomError(licenseToken, "LockedBalanceInvariant");

      await time.increaseTo(tge + YEAR);
      await licenseToken.connect(beneficiary).transfer(recipientAddress, 1n);
    });
  });
});
