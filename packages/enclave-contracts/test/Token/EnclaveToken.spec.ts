// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import { network } from "hardhat";

import { EnclaveToken__factory as EnclaveTokenFactory } from "../../types";

const { ethers, networkHelpers } = await network.connect();
const { loadFixture, time } = networkHelpers;

describe("EnclaveToken", function () {
  async function deploy() {
    const [deployer, admin, minter, whitelister, alice, bob] =
      await ethers.getSigners();
    const token = await new EnclaveTokenFactory(deployer).deploy(
      await admin.getAddress(),
    );
    return { deployer, admin, minter, whitelister, alice, bob, token };
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
});
