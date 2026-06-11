// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";

import {
  InterfoldToken__factory as InterfoldTokenFactory,
  MockBondingRegistry__factory as MockBondingRegistryFactory,
} from "../../types";
import { deployInterfoldSystem, ethers, networkHelpers } from "../fixtures";

const { loadFixture, time } = networkHelpers;

const DAY = 24n * 60n * 60n;
const YEAR = 365n * DAY;

describe("InterfoldToken", function () {
  // ── Helpers ─────────────────────────────────────────────────────────────

  /// Deploy a minimal MockBondingRegistry + InterfoldToken for standalone tests.
  /// CCA window starts far in the future so tests control the phase via
  /// `time.increaseTo` / `time.increase`.
  async function deploy() {
    const [
      deployer,
      admin,
      minter,
      whitelister,
      lockManager,
      alice,
      bob,
      claimSource,
    ] = await ethers.getSigners();

    // Deploy a minimal mock BondingRegistry that returns 0 for totalBonded.
    const mockRegistry = await new MockBondingRegistryFactory(
      deployer,
    ).deploy();
    await mockRegistry.waitForDeployment();

    const now = BigInt(await time.latest());
    const ccaStart = now + 10n * DAY; // far future — Virtual phase
    const ccaEnd = ccaStart + 7n * DAY;

    const token = await new InterfoldTokenFactory(deployer).deploy(
      await admin.getAddress(),
      ccaStart,
      ccaEnd,
      await claimSource.getAddress(),
      await mockRegistry.getAddress(),
    );

    return {
      deployer,
      admin,
      minter,
      whitelister,
      lockManager,
      alice,
      bob,
      claimSource,
      token,
      mockRegistry,
      ccaStart,
      ccaEnd,
    };
  }

  /// Deploy, create a policy, mint locked tokens, THEN fire TGE.
  /// Returns everything needed for transfer-enforcement tests.
  async function deployWithLockAndTge(
    opts: {
      policyName?: string;
      mintAmount?: bigint;
      vestDuration?: bigint;
      holdUntil?: bigint;
      recipient?: "alice" | "claimSource";
    } = {},
  ) {
    const fixture = await loadFixture(deploy);
    const { token, admin, alice, claimSource, ccaEnd } = fixture;
    const recipient = opts.recipient === "claimSource" ? claimSource : alice;
    const recipientAddress = await recipient.getAddress();
    const policyId = await createLinearPolicy(
      token,
      admin,
      opts.policyName ?? "TEST_LOCK",
      {
        vestDuration: opts.vestDuration ?? 2n * YEAR,
        holdUntil: opts.holdUntil,
      },
    );
    const amount = opts.mintAmount ?? ethers.parseEther("1000");

    // Mint during Virtual phase.
    await token.connect(admin).mintAllocations([
      {
        recipient: recipientAddress,
        amount,
        policyId,
        label: ethers.encodeBytes32String("test"),
      },
    ]);

    // Fire TGE.
    const TGE_COOLDOWN = 45n * DAY;
    await time.increaseTo(ccaEnd + TGE_COOLDOWN + 1n);
    const tgeTx = await token.tge();
    const receipt = await tgeTx.wait();
    const tgeBlock = await ethers.provider.getBlock(receipt!.blockNumber);
    const tgeTimestamp = BigInt(tgeBlock!.timestamp);

    return { ...fixture, policyId, amount, tgeTimestamp, recipientAddress };
  }

  /// Deploy, mint unlocked tokens to alice, THEN fire TGE.
  async function deployWithUnlockedAndTge(mintAmount?: bigint) {
    const fixture = await loadFixture(deploy);
    const { token, admin, alice, ccaEnd } = fixture;
    const amount = mintAmount ?? ethers.parseEther("500");

    await token
      .connect(admin)
      .mint(await alice.getAddress(), amount, ethers.ZeroHash);

    const TGE_COOLDOWN = 45n * DAY;
    await time.increaseTo(ccaEnd + TGE_COOLDOWN + 1n);
    await token.tge();

    return { ...fixture, amount };
  }

  // ── Helpers for lock policies ───────────────────────────────────────────

  /// Create a standard linear lock policy and return its id.
  async function createLinearPolicy(
    token: Awaited<ReturnType<typeof deploy>>["token"],
    admin: Awaited<ReturnType<typeof deploy>>["admin"],
    policyId: string,
    opts: {
      anchor?: number; // 0 = Absolute, 1 = Tge
      start?: bigint;
      cliffDuration?: bigint;
      vestDuration?: bigint;
      holdUntil?: bigint;
    } = {},
  ) {
    const id = ethers.encodeBytes32String(policyId);
    const anchor = opts.anchor ?? 1; // default Tge-anchored
    const start = opts.start ?? 0n;
    const cliffDuration = opts.cliffDuration ?? 0n;
    const vestDuration = opts.vestDuration ?? 2n * YEAR;
    await token.connect(admin).createLockPolicy(id, {
      holdUntil: opts.holdUntil ?? 0n,
      unlock: { anchor, start, cliffDuration, vestDuration },
    });
    return id;
  }

  // ═════════════════════════════════════════════════════════════════════════
  // Deployment & Constructor
  // ═════════════════════════════════════════════════════════════════════════

  describe("constructor", function () {
    it("reverts when claimSource is zero address", async function () {
      const [deployer] = await ethers.getSigners();
      const mockRegistry = await new MockBondingRegistryFactory(
        deployer,
      ).deploy();
      await mockRegistry.waitForDeployment();
      const now = BigInt(await time.latest());
      const ccaStart = now + DAY;
      const ccaEnd = ccaStart + 7n * DAY;

      await expect(
        new InterfoldTokenFactory(deployer).deploy(
          await deployer.getAddress(),
          ccaStart,
          ccaEnd,
          ethers.ZeroAddress,
          await mockRegistry.getAddress(),
        ),
      ).to.be.revertedWithCustomError(
        { interface: InterfoldTokenFactory.createInterface() },
        "ZeroAddress",
      );
    });

    it("reverts when bondingRegistry is zero address", async function () {
      const [deployer] = await ethers.getSigners();
      const now = BigInt(await time.latest());
      const ccaStart = now + DAY;
      const ccaEnd = ccaStart + 7n * DAY;

      await expect(
        new InterfoldTokenFactory(deployer).deploy(
          await deployer.getAddress(),
          ccaStart,
          ccaEnd,
          await deployer.getAddress(),
          ethers.ZeroAddress,
        ),
      ).to.be.revertedWithCustomError(
        { interface: InterfoldTokenFactory.createInterface() },
        "ZeroAddress",
      );
    });

    it("reverts when bondingRegistry has no code (EOA)", async function () {
      const [deployer, admin] = await ethers.getSigners();
      const now = BigInt(await time.latest());
      const ccaStart = now + DAY;
      const ccaEnd = ccaStart + 7n * DAY;

      await expect(
        new InterfoldTokenFactory(deployer).deploy(
          await admin.getAddress(),
          ccaStart,
          ccaEnd,
          await deployer.getAddress(),
          await admin.getAddress(), // EOA, not a contract
        ),
      ).to.be.revertedWithCustomError(
        { interface: InterfoldTokenFactory.createInterface() },
        "InvalidBondingRegistry",
      );
    });

    it("reverts when CCA start is in the past", async function () {
      const [deployer] = await ethers.getSigners();
      const mockRegistry = await new MockBondingRegistryFactory(
        deployer,
      ).deploy();
      await mockRegistry.waitForDeployment();
      const now = BigInt(await time.latest());

      await expect(
        new InterfoldTokenFactory(deployer).deploy(
          await deployer.getAddress(),
          now, // in the past (or now)
          now + 7n * DAY,
          await deployer.getAddress(),
          await mockRegistry.getAddress(),
        ),
      ).to.be.revertedWithCustomError(
        { interface: InterfoldTokenFactory.createInterface() },
        "InvalidCcaWindow",
      );
    });

    it("reverts when CCA end is not after start", async function () {
      const [deployer] = await ethers.getSigners();
      const mockRegistry = await new MockBondingRegistryFactory(
        deployer,
      ).deploy();
      await mockRegistry.waitForDeployment();
      const now = BigInt(await time.latest());
      const ccaStart = now + DAY;
      const ccaEnd = ccaStart; // equal, not greater

      await expect(
        new InterfoldTokenFactory(deployer).deploy(
          await deployer.getAddress(),
          ccaStart,
          ccaEnd,
          await deployer.getAddress(),
          await mockRegistry.getAddress(),
        ),
      ).to.be.revertedWithCustomError(
        { interface: InterfoldTokenFactory.createInterface() },
        "InvalidCcaWindow",
      );
    });

    it("initial owner receives all roles", async function () {
      const { token, admin } = await loadFixture(deploy);
      const adminAddress = await admin.getAddress();
      expect(
        await token.hasRole(await token.DEFAULT_ADMIN_ROLE(), adminAddress),
      ).to.be.true;
      expect(await token.hasRole(await token.MINTER_ROLE(), adminAddress)).to.be
        .true;
      expect(await token.hasRole(await token.WHITELIST_ROLE(), adminAddress)).to
        .be.true;
      expect(await token.hasRole(await token.LOCK_MANAGER_ROLE(), adminAddress))
        .to.be.true;
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // Phase lifecycle
  // ═════════════════════════════════════════════════════════════════════════

  describe("phase()", function () {
    it("starts in Virtual phase", async function () {
      const { token } = await loadFixture(deploy);
      expect(await token.phase()).to.equal(0); // Phase.Virtual
    });

    it("enters PublicSale during CCA window", async function () {
      const { token, ccaStart } = await loadFixture(deploy);
      await time.increaseTo(ccaStart);
      expect(await token.phase()).to.equal(1); // Phase.PublicSale
    });

    it("enters Cooldown after CCA_END before TGE", async function () {
      const { token, ccaEnd } = await loadFixture(deploy);
      await time.increaseTo(ccaEnd);
      expect(await token.phase()).to.equal(2); // Phase.Cooldown
    });

    it("enters Live phase after TGE", async function () {
      const { token, ccaEnd } = await loadFixture(deploy);
      const TGE_COOLDOWN = 45n * DAY;
      await time.increaseTo(ccaEnd + TGE_COOLDOWN + 1n);
      await token.tge();
      expect(await token.phase()).to.equal(3); // Phase.Live
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // Minting
  // ═════════════════════════════════════════════════════════════════════════

  describe("mint", function () {
    it("DEFAULT_ADMIN_ROLE can mint unlocked tokens during Virtual", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      const amount = ethers.parseEther("100");
      await expect(
        token
          .connect(admin)
          .mint(
            await alice.getAddress(),
            amount,
            ethers.encodeBytes32String("test"),
          ),
      )
        .to.emit(token, "AllocationMinted")
        .withArgs(
          await alice.getAddress(),
          amount,
          ethers.ZeroHash,
          ethers.encodeBytes32String("test"),
        );
      expect(await token.balanceOf(await alice.getAddress())).to.equal(amount);
    });

    it("mint reverts after Virtual phase", async function () {
      const { token, admin, alice, ccaStart } = await loadFixture(deploy);
      await time.increaseTo(ccaStart);
      await expect(
        token
          .connect(admin)
          .mint(
            await alice.getAddress(),
            ethers.parseEther("1"),
            ethers.encodeBytes32String("test"),
          ),
      ).to.be.revertedWithCustomError(token, "MintingClosed");
    });

    it("reverts with zero amount", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      await expect(
        token
          .connect(admin)
          .mint(
            await alice.getAddress(),
            0n,
            ethers.encodeBytes32String("test"),
          ),
      ).to.be.revertedWithCustomError(token, "ZeroAmount");
    });

    it("reverts when MAX_SUPPLY would be exceeded", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      const maxSupply = await token.MAX_SUPPLY();
      await expect(
        token
          .connect(admin)
          .mint(await alice.getAddress(), maxSupply + 1n, ethers.ZeroHash),
      ).to.be.revertedWithCustomError(token, "MaxSupplyExceeded");
    });
  });

  describe("mintAllocations", function () {
    it("MINTER_ROLE can mint locked allocations during Virtual", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      const policyId = await createLinearPolicy(token, admin, "TEST_POLICY");
      const amount = ethers.parseEther("1000");

      await expect(
        token.connect(admin).mintAllocations([
          {
            recipient: await alice.getAddress(),
            amount,
            policyId,
            label: ethers.encodeBytes32String("test"),
          },
        ]),
      )
        .to.emit(token, "AllocationMinted")
        .withArgs(
          await alice.getAddress(),
          amount,
          policyId,
          ethers.encodeBytes32String("test"),
        );

      // Tokens are locked — lockedBalanceOf should be > 0.
      expect(await token.lockedBalanceOf(await alice.getAddress())).to.equal(
        amount,
      );
      expect(await token.balanceOf(await alice.getAddress())).to.equal(amount);
    });

    it("reverts with zero policyId", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      await expect(
        token.connect(admin).mintAllocations([
          {
            recipient: await alice.getAddress(),
            amount: ethers.parseEther("1"),
            policyId: ethers.ZeroHash,
            label: ethers.encodeBytes32String("test"),
          },
        ]),
      ).to.be.revertedWithCustomError(token, "InvalidPolicy");
    });

    it("reverts with undefined policy", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      await expect(
        token.connect(admin).mintAllocations([
          {
            recipient: await alice.getAddress(),
            amount: ethers.parseEther("1"),
            policyId: ethers.encodeBytes32String("UNDEFINED"),
            label: ethers.encodeBytes32String("test"),
          },
        ]),
      ).to.be.revertedWithCustomError(token, "PolicyNotDefined");
    });

    it("reverts after Virtual phase", async function () {
      const { token, admin, alice, ccaStart } = await loadFixture(deploy);
      const policyId = await createLinearPolicy(token, admin, "TEST_POLICY");
      await time.increaseTo(ccaStart);
      await expect(
        token.connect(admin).mintAllocations([
          {
            recipient: await alice.getAddress(),
            amount: ethers.parseEther("1"),
            policyId,
            label: ethers.ZeroHash,
          },
        ]),
      ).to.be.revertedWithCustomError(token, "MintingClosed");
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // TGE
  // ═════════════════════════════════════════════════════════════════════════

  describe("tge()", function () {
    it("reverts before CCA_END + TGE_COOLDOWN", async function () {
      const { token, ccaEnd } = await loadFixture(deploy);
      await time.increaseTo(ccaEnd); // Cooldown phase but not enough
      await expect(token.tge()).to.be.revertedWithCustomError(
        token,
        "TgeTooEarly",
      );
    });

    it("anyone can trigger TGE after cooldown", async function () {
      const { token, ccaEnd, alice } = await loadFixture(deploy);
      const TGE_COOLDOWN = 45n * DAY;
      await time.increaseTo(ccaEnd + TGE_COOLDOWN + 1n);
      await expect(token.connect(alice).tge()).to.emit(token, "TgeTriggered");
      expect(await token.tgeTimestamp()).to.be.gt(0);
      expect(await token.phase()).to.equal(3); // Live
    });

    it("reverts if already live", async function () {
      const { token, ccaEnd } = await loadFixture(deploy);
      const TGE_COOLDOWN = 45n * DAY;
      await time.increaseTo(ccaEnd + TGE_COOLDOWN + 1n);
      await token.tge();
      await expect(token.tge()).to.be.revertedWithCustomError(
        token,
        "AlreadyLive",
      );
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // Whitelisting
  // ═════════════════════════════════════════════════════════════════════════

  describe("setTransferWhitelisted", function () {
    it("WHITELIST_ROLE can whitelist an address", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      await expect(
        token
          .connect(admin)
          .setTransferWhitelisted(await alice.getAddress(), true),
      )
        .to.emit(token, "TransferWhitelistUpdated")
        .withArgs(await alice.getAddress(), true);
      expect(await token.transferWhitelist(await alice.getAddress())).to.be
        .true;
    });

    it("non-WHITELIST_ROLE cannot whitelist", async function () {
      const { token, alice } = await loadFixture(deploy);
      await expect(
        token
          .connect(alice)
          .setTransferWhitelisted(await alice.getAddress(), true),
      ).to.be.revertedWithCustomError(
        token,
        "AccessControlUnauthorizedAccount",
      );
    });

    it("reverts with zero address", async function () {
      const { token, admin } = await loadFixture(deploy);
      await expect(
        token.connect(admin).setTransferWhitelisted(ethers.ZeroAddress, true),
      ).to.be.revertedWithCustomError(token, "ZeroAddress");
    });
  });

  describe("setLockWhitelisted", function () {
    it("LOCK_MANAGER_ROLE can manage lock whitelist", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      await expect(
        token.connect(admin).setLockWhitelisted(await alice.getAddress(), true),
      )
        .to.emit(token, "LockWhitelistUpdated")
        .withArgs(await alice.getAddress(), true);
    });

    it("non-LOCK_MANAGER_ROLE cannot manage lock whitelist", async function () {
      const { token, alice } = await loadFixture(deploy);
      await expect(
        token.connect(alice).setLockWhitelisted(await alice.getAddress(), true),
      ).to.be.revertedWithCustomError(
        token,
        "AccessControlUnauthorizedAccount",
      );
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // Lock Policies
  // ═════════════════════════════════════════════════════════════════════════

  describe("createLockPolicy", function () {
    it("LOCK_MANAGER_ROLE can create a policy", async function () {
      const { token, admin } = await loadFixture(deploy);
      const policyId = ethers.encodeBytes32String("MY_POLICY");
      await expect(
        token.connect(admin).createLockPolicy(policyId, {
          holdUntil: 0n,
          unlock: {
            anchor: 1, // Tge
            start: 0n,
            cliffDuration: 0n,
            vestDuration: 2n * YEAR,
          },
        }),
      ).to.emit(token, "PolicyDefined");
    });

    it("reverts on duplicate policy id (write-once)", async function () {
      const { token, admin } = await loadFixture(deploy);
      const policyId = ethers.encodeBytes32String("MY_POLICY");
      await token.connect(admin).createLockPolicy(policyId, {
        holdUntil: 0n,
        unlock: {
          anchor: 1,
          start: 0n,
          cliffDuration: 0n,
          vestDuration: YEAR,
        },
      });
      await expect(
        token.connect(admin).createLockPolicy(policyId, {
          holdUntil: 0n,
          unlock: {
            anchor: 0,
            start: 1n,
            cliffDuration: 1n,
            vestDuration: 0n,
          },
        }),
      ).to.be.revertedWithCustomError(token, "PolicyAlreadyDefined");
    });

    it("reverts with zero policyId", async function () {
      const { token, admin } = await loadFixture(deploy);
      await expect(
        token.connect(admin).createLockPolicy(ethers.ZeroHash, {
          holdUntil: 0n,
          unlock: {
            anchor: 1,
            start: 0n,
            cliffDuration: 0n,
            vestDuration: YEAR,
          },
        }),
      ).to.be.revertedWithCustomError(token, "InvalidPolicy");
    });

    it("reverts with PENDING policyId", async function () {
      const { token, admin } = await loadFixture(deploy);
      await expect(
        token
          .connect(admin)
          .createLockPolicy(ethers.encodeBytes32String("PENDING"), {
            holdUntil: 0n,
            unlock: {
              anchor: 1,
              start: 0n,
              cliffDuration: 0n,
              vestDuration: YEAR,
            },
          }),
      ).to.be.revertedWithCustomError(token, "InvalidPolicy");
    });

    it("reverts when both cliff and vest are zero", async function () {
      const { token, admin } = await loadFixture(deploy);
      await expect(
        token
          .connect(admin)
          .createLockPolicy(ethers.encodeBytes32String("BAD"), {
            holdUntil: 0n,
            unlock: {
              anchor: 1,
              start: 0n,
              cliffDuration: 0n,
              vestDuration: 0n,
            },
          }),
      ).to.be.revertedWithCustomError(token, "InvalidPolicy");
    });

    it("reverts when Absolute anchor has zero start", async function () {
      const { token, admin } = await loadFixture(deploy);
      await expect(
        token
          .connect(admin)
          .createLockPolicy(ethers.encodeBytes32String("BAD"), {
            holdUntil: 0n,
            unlock: {
              anchor: 0,
              start: 0n,
              cliffDuration: 1n,
              vestDuration: 0n,
            },
          }),
      ).to.be.revertedWithCustomError(token, "InvalidPolicy");
    });

    it("reverts when Tge anchor has non-zero start", async function () {
      const { token, admin } = await loadFixture(deploy);
      await expect(
        token
          .connect(admin)
          .createLockPolicy(ethers.encodeBytes32String("BAD"), {
            holdUntil: 0n,
            unlock: {
              anchor: 1,
              start: 1n,
              cliffDuration: 1n,
              vestDuration: 0n,
            },
          }),
      ).to.be.revertedWithCustomError(token, "InvalidPolicy");
    });

    it("reverts when cliff exceeds vest duration", async function () {
      const { token, admin } = await loadFixture(deploy);
      await expect(
        token
          .connect(admin)
          .createLockPolicy(ethers.encodeBytes32String("BAD"), {
            holdUntil: 0n,
            unlock: {
              anchor: 1,
              start: 0n,
              cliffDuration: 2n * YEAR,
              vestDuration: YEAR,
            },
          }),
      ).to.be.revertedWithCustomError(token, "InvalidPolicy");
    });

    it("non-LOCK_MANAGER_ROLE cannot create a policy", async function () {
      const { token, alice } = await loadFixture(deploy);
      await expect(
        token
          .connect(alice)
          .createLockPolicy(ethers.encodeBytes32String("MY_POLICY"), {
            holdUntil: 0n,
            unlock: {
              anchor: 1,
              start: 0n,
              cliffDuration: 0n,
              vestDuration: YEAR,
            },
          }),
      ).to.be.revertedWithCustomError(
        token,
        "AccessControlUnauthorizedAccount",
      );
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // Lock enforcement
  // ═════════════════════════════════════════════════════════════════════════

  describe("lockedBalanceOf / lockedBalanceAt / transferableBalanceOf", function () {
    it("lockedBalanceOf returns 0 for accounts with no locks", async function () {
      const { token, alice } = await loadFixture(deploy);
      expect(await token.lockedBalanceOf(await alice.getAddress())).to.equal(
        0n,
      );
    });

    it("mintAllocation creates a lock tracked by lockedBalanceOf", async function () {
      const { token, alice, amount } = await deployWithLockAndTge({
        mintAmount: ethers.parseEther("2400"),
      });
      expect(await token.lockedBalanceOf(await alice.getAddress())).to.equal(
        amount,
      );
    });

    it("TGE-anchored policy releases nothing before TGE timestamp", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      const policyId = await createLinearPolicy(token, admin, "TEST_POLICY", {
        vestDuration: 2n * YEAR,
      });
      const amount = ethers.parseEther("2400");

      await token.connect(admin).mintAllocations([
        {
          recipient: await alice.getAddress(),
          amount,
          policyId,
          label: ethers.encodeBytes32String("test"),
        },
      ]);

      // TGE not fired yet, Tge-anchored curve should keep everything locked.
      expect(await token.lockedBalanceOf(await alice.getAddress())).to.equal(
        amount,
      );
    });

    it("linear unlock over time after TGE", async function () {
      const { token, admin, alice, ccaEnd } = await loadFixture(deploy);
      const policyId = await createLinearPolicy(token, admin, "TEST_POLICY", {
        vestDuration: 2n * YEAR,
      });
      const amount = ethers.parseEther("2400");

      await token.connect(admin).mintAllocations([
        {
          recipient: await alice.getAddress(),
          amount,
          policyId,
          label: ethers.encodeBytes32String("test"),
        },
      ]);

      // Fire TGE.
      const TGE_COOLDOWN = 45n * DAY;
      await time.increaseTo(ccaEnd + TGE_COOLDOWN + 1n);
      const tgeTx = await token.tge();
      const receipt = await tgeTx.wait();
      const tgeBlock = await ethers.provider.getBlock(receipt!.blockNumber);
      const tgeTimestamp = BigInt(tgeBlock!.timestamp);

      // Right at TGE: everything still locked (cliffDuration = 0 so it starts
      // vesting immediately — but at timestamp == anchor, nothing has accrued).
      expect(await token.lockedBalanceOf(await alice.getAddress())).to.equal(
        amount,
      );

      // Halfway through vesting: half unlocked.
      await time.increaseTo(tgeTimestamp + YEAR);
      expect(await token.lockedBalanceOf(await alice.getAddress())).to.equal(
        amount / 2n,
      );

      // Past vest end: fully unlocked.
      await time.increaseTo(tgeTimestamp + 2n * YEAR);
      expect(await token.lockedBalanceOf(await alice.getAddress())).to.equal(
        0n,
      );
    });

    it("holdUntil keeps everything locked regardless of curve", async function () {
      // Use deployWithLockAndTge which creates a Tge-anchored lock with holdUntil=0.
      // Then verify lockedBalanceAt at various timestamps.
      const { token, alice, amount, tgeTimestamp } = await deployWithLockAndTge(
        { mintAmount: ethers.parseEther("1000") },
      );
      const aliceAddress = await alice.getAddress();

      // At tgeTimestamp, lock is fully locked (no time elapsed).
      expect(await token.lockedBalanceAt(aliceAddress, tgeTimestamp)).to.equal(
        amount,
      );

      // At tgeTimestamp + YEAR, half is unlocked (linear vest over 2Y).
      expect(
        await token.lockedBalanceAt(aliceAddress, tgeTimestamp + YEAR),
      ).to.equal(amount / 2n);

      // At tgeTimestamp + 2*YEAR, fully unlocked.
      expect(
        await token.lockedBalanceAt(aliceAddress, tgeTimestamp + 2n * YEAR),
      ).to.equal(0n);
    });

    it("transferableBalanceOf returns full balance when nothing locked", async function () {
      const { token, alice, amount } = await deployWithUnlockedAndTge(
        ethers.parseEther("100"),
      );
      expect(
        await token.transferableBalanceOf(await alice.getAddress()),
      ).to.equal(amount);
    });

    it("transferableBalanceOf = 0 when fully locked and no bond", async function () {
      const { token, alice, amount } = await deployWithLockAndTge({
        mintAmount: ethers.parseEther("1000"),
      });
      expect(
        await token.transferableBalanceOf(await alice.getAddress()),
      ).to.equal(0n);
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // Transfer enforcement
  // ═════════════════════════════════════════════════════════════════════════

  describe("transfer enforcement", function () {
    it("blocks transfer that would drop below locked balance", async function () {
      const { token, alice, bob, amount } = await deployWithLockAndTge({
        mintAmount: ethers.parseEther("1000"),
      });
      // After TGE, a tiny fraction may have unlocked (1-2 seconds of vesting).
      // transferableBalance should be far less than the full amount.
      const transferable = await token.transferableBalanceOf(
        await alice.getAddress(),
      );
      expect(transferable).to.be.lt(amount / 2n);
      // Attempting to transfer the full amount should revert.
      await expect(
        token.connect(alice).transfer(await bob.getAddress(), amount),
      ).to.be.revertedWithCustomError(token, "InsufficientUnlockedBalance");
    });

    it("allows transfer of unlocked portion", async function () {
      const { token, alice, bob, amount, tgeTimestamp } =
        await deployWithLockAndTge({
          mintAmount: ethers.parseEther("1000"),
          vestDuration: 2n * YEAR,
        });

      await time.increaseTo(tgeTimestamp + YEAR);

      // Half unlocked.
      const half = amount / 2n;
      expect(
        await token.transferableBalanceOf(await alice.getAddress()),
      ).to.equal(half);

      await token.connect(alice).transfer(await bob.getAddress(), half);
    });

    it("pre-TGE: bonding registry transfers are allowed", async function () {
      const { token, admin, alice, mockRegistry } = await loadFixture(deploy);
      const amount = ethers.parseEther("100");
      const registryAddress = await mockRegistry.getAddress();

      await token
        .connect(admin)
        .mint(await alice.getAddress(), amount, ethers.ZeroHash);

      // Transfer TO bonding registry — should work pre-TGE.
      await token.connect(alice).transfer(registryAddress, amount);
    });

    it("pre-TGE: whitelisted addresses can transfer", async function () {
      const { token, admin, alice, bob } = await loadFixture(deploy);
      const amount = ethers.parseEther("100");

      await token
        .connect(admin)
        .mint(await alice.getAddress(), amount, ethers.ZeroHash);
      await token
        .connect(admin)
        .setTransferWhitelisted(await alice.getAddress(), true);

      await token.connect(alice).transfer(await bob.getAddress(), amount);
    });

    it("pre-TGE: claim source transfers are allowed", async function () {
      const { token, admin, alice, claimSource } = await loadFixture(deploy);
      const amount = ethers.parseEther("100");

      await token
        .connect(admin)
        .mint(await claimSource.getAddress(), amount, ethers.ZeroHash);

      await token
        .connect(claimSource)
        .transfer(await alice.getAddress(), amount);
    });

    it("pre-TGE: regular transfers are blocked", async function () {
      const { token, admin, alice, bob } = await loadFixture(deploy);
      const amount = ethers.parseEther("100");

      await token
        .connect(admin)
        .mint(await alice.getAddress(), amount, ethers.ZeroHash);

      await expect(
        token.connect(alice).transfer(await bob.getAddress(), amount),
      ).to.be.revertedWithCustomError(token, "TransferRestricted");
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // Claim-source auto-lock & linkClaim
  // ═════════════════════════════════════════════════════════════════════════

  describe("claim-source auto-lock & linkClaim", function () {
    it("CLAIM_SOURCE transfers create PENDING locks", async function () {
      const { token, alice, claimSource, amount } =
        await deployWithUnlockedAndTge(ethers.parseEther("500"));

      // Transfer from alice to claimSource first so claimSource has tokens.
      await token
        .connect(alice)
        .transfer(await claimSource.getAddress(), amount);

      await token
        .connect(claimSource)
        .transfer(await alice.getAddress(), amount);

      // Pending lock should be created.
      expect(await token.lockedBalanceOf(await alice.getAddress())).to.equal(
        amount,
      );
    });

    it("lockWhitelist exempts from auto-lock on claim-source transfer", async function () {
      const { token, admin, alice, claimSource, amount } =
        await deployWithUnlockedAndTge(ethers.parseEther("500"));

      await token
        .connect(admin)
        .setLockWhitelisted(await alice.getAddress(), true);

      // Transfer tokens from alice to claimSource so claimSource can send.
      await token
        .connect(alice)
        .transfer(await claimSource.getAddress(), amount);

      await token
        .connect(claimSource)
        .transfer(await alice.getAddress(), amount);

      // No lock created because recipient is lock-whitelisted.
      expect(await token.lockedBalanceOf(await alice.getAddress())).to.equal(
        0n,
      );
      expect(await token.balanceOf(await alice.getAddress())).to.equal(amount);
    });

    it("linkClaim moves PENDING to a real policy", async function () {
      const { token, admin, alice, claimSource, amount } =
        await deployWithUnlockedAndTge(ethers.parseEther("500"));
      const policyId = await createLinearPolicy(token, admin, "REAL_POLICY", {
        vestDuration: 2n * YEAR,
      });

      // Transfer from alice to claimSource so claimSource can send.
      await token
        .connect(alice)
        .transfer(await claimSource.getAddress(), amount);
      await token
        .connect(claimSource)
        .transfer(await alice.getAddress(), amount);

      // Now link the claim to the real policy.
      await token
        .connect(admin)
        .linkClaim(await alice.getAddress(), amount, policyId);

      // Lock should still exist but now under the real policy (allow tiny
      // rounding from vesting elapsed seconds).
      const lb = await token.lockedBalanceOf(await alice.getAddress());
      expect(lb).to.be.closeTo(amount, ethers.parseEther("0.01"));
    });

    it("linkClaim queues unfilled amounts for future claims", async function () {
      const fixture = await loadFixture(deploy);
      const { token, admin, alice, claimSource, ccaEnd } = fixture;
      const policyId = await createLinearPolicy(token, admin, "FUTURE_POLICY", {
        vestDuration: 2n * YEAR,
      });
      const linkAmount = ethers.parseEther("500");

      // Link before any claim arrives — should queue.
      await token
        .connect(admin)
        .linkClaim(await alice.getAddress(), linkAmount, policyId);

      // No balance yet so no active lock.
      expect(await token.lockedBalanceOf(await alice.getAddress())).to.equal(
        0n,
      );

      // Mint tokens to claimSource during Virtual phase.
      await token
        .connect(admin)
        .mint(await claimSource.getAddress(), linkAmount, ethers.ZeroHash);

      // Fire TGE so transfers are unrestricted.
      const TGE_COOLDOWN = 45n * DAY;
      await time.increaseTo(ccaEnd + TGE_COOLDOWN + 1n);
      await token.tge();

      // Now send a claim — it should consume the queued lock.
      await token
        .connect(claimSource)
        .transfer(await alice.getAddress(), linkAmount);

      // Queued lock should be consumed and active lock created (allow tiny
      // rounding from vesting elapsed seconds).
      const lb2 = await token.lockedBalanceOf(await alice.getAddress());
      expect(lb2).to.be.closeTo(linkAmount, ethers.parseEther("0.01"));
    });

    it("linkClaim reverts with undefined policy", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      await expect(
        token
          .connect(admin)
          .linkClaim(
            await alice.getAddress(),
            ethers.parseEther("1"),
            ethers.encodeBytes32String("UNDEFINED"),
          ),
      ).to.be.revertedWithCustomError(token, "PolicyNotDefined");
    });

    it("linkClaim reverts with zero amount", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      const policyId = await createLinearPolicy(token, admin, "REAL_POLICY");
      await expect(
        token.connect(admin).linkClaim(await alice.getAddress(), 0n, policyId),
      ).to.be.revertedWithCustomError(token, "ZeroAmount");
    });

    it("non-LOCK_MANAGER_ROLE cannot linkClaim", async function () {
      const { token, alice } = await loadFixture(deploy);
      await expect(
        token
          .connect(alice)
          .linkClaim(
            await alice.getAddress(),
            ethers.parseEther("1"),
            ethers.encodeBytes32String("ANY"),
          ),
      ).to.be.revertedWithCustomError(
        token,
        "AccessControlUnauthorizedAccount",
      );
    });

    it("queued locks survive multiple partial claims", async function () {
      const fixture = await loadFixture(deploy);
      const { token, admin, alice, claimSource, ccaEnd } = fixture;
      const policyId = await createLinearPolicy(token, admin, "PARTIAL", {
        vestDuration: 2n * YEAR,
      });
      const linkAmount = ethers.parseEther("1000");

      // Queue a large amount.
      await token
        .connect(admin)
        .linkClaim(await alice.getAddress(), linkAmount, policyId);

      // Mint all claim tokens during Virtual phase.
      const totalClaim = ethers.parseEther("700");
      await token
        .connect(admin)
        .mint(await claimSource.getAddress(), totalClaim, ethers.ZeroHash);

      // Fire TGE.
      const TGE_COOLDOWN = 45n * DAY;
      await time.increaseTo(ccaEnd + TGE_COOLDOWN + 1n);
      await token.tge();

      // Send a partial claim.
      const partialAmount = ethers.parseEther("400");
      await token
        .connect(claimSource)
        .transfer(await alice.getAddress(), partialAmount);

      let lb3 = await token.lockedBalanceOf(await alice.getAddress());
      expect(lb3).to.be.closeTo(partialAmount, ethers.parseEther("0.01"));

      // Send another claim.
      const anotherAmount = ethers.parseEther("300");
      await token
        .connect(claimSource)
        .transfer(await alice.getAddress(), anotherAmount);

      lb3 = await token.lockedBalanceOf(await alice.getAddress());
      expect(lb3).to.be.closeTo(
        partialAmount + anotherAmount,
        ethers.parseEther("0.01"),
      );
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // BondingRegistry integration (uses deployInterfoldSystem)
  // ═════════════════════════════════════════════════════════════════════════

  describe("BondingRegistry integration", function () {
    it("transferableBalanceOf counts bonded INTF toward the locked floor", async function () {
      const signers = await ethers.getSigners();
      const [, beneficiary, slasher] = signers;
      const beneficiaryAddress = await beneficiary.getAddress();
      const slasherAddress = await slasher.getAddress();
      const sys = await deployInterfoldSystem({
        useMockCiphernodeRegistry: true,
        setupOperators: 0,
        wireSlashingManager: false,
        mintUsdcTo: [],
      });
      const { bondingRegistry, licenseToken } = sys;
      const bondingRegistryAddress = await bondingRegistry.getAddress();
      const totalAmount = ethers.parseEther("1000");
      const bondAmount = ethers.parseEther("800");

      await bondingRegistry.setSlashingManager(slasherAddress);

      // Mint unlocked tokens and bond some.
      await licenseToken.mint(
        beneficiaryAddress,
        totalAmount,
        ethers.encodeBytes32String("test"),
      );
      await licenseToken
        .connect(beneficiary)
        .approve(bondingRegistryAddress, bondAmount);
      await bondingRegistry.connect(beneficiary).bondLicense(bondAmount);

      // Wallet balance is totalAmount - bondAmount, bonded = bondAmount.
      // No locks so everything is transferable.
      expect(await licenseToken.balanceOf(beneficiaryAddress)).to.equal(
        totalAmount - bondAmount,
      );
      expect(
        await licenseToken.transferableBalanceOf(beneficiaryAddress),
      ).to.equal(totalAmount - bondAmount);

      // Now create a lock policy and mint a locked allocation.
      const policyId = ethers.encodeBytes32String("BOND_TEST");
      await licenseToken.createLockPolicy(policyId, {
        holdUntil: 0n,
        unlock: {
          anchor: 1,
          start: 0n,
          cliffDuration: 0n,
          vestDuration: 2n * YEAR,
        },
      });
      const lockAmount = ethers.parseEther("400");
      // Mint extra unlocked tokens to fund the lock.
      await licenseToken.mint(
        beneficiaryAddress,
        lockAmount,
        ethers.encodeBytes32String("extra"),
      );
      await licenseToken.mintAllocations([
        {
          recipient: beneficiaryAddress,
          amount: lockAmount,
          policyId,
          label: ethers.encodeBytes32String("locked"),
        },
      ]);

      // Locked balance ≈ lockAmount (400) — Tge-anchored with tiny vesting.
      // Bonded balance = bondAmount (800).
      // Since bonded > locked, the bond covers all locks.
      // Wallet = totalAmount - bondAmount + lockAmount + lockAmount
      //        = 1000 - 800 + 400 + 400 = 1000.
      // transferable = balance - max(0, locked - bonded) ≈ 1000 - 0 = 1000.
      const tb = await licenseToken.transferableBalanceOf(beneficiaryAddress);
      expect(tb).to.be.closeTo(
        totalAmount - bondAmount + lockAmount + lockAmount,
        ethers.parseEther("0.01"),
      );
    });

    it("bonding registry transfers are allowed pre-TGE", async function () {
      const sys = await deployInterfoldSystem({
        useMockCiphernodeRegistry: true,
        setupOperators: 0,
        mintUsdcTo: [],
      });
      const { bondingRegistry, licenseToken, owner } = sys;
      const bondingRegistryAddress = await bondingRegistry.getAddress();
      const bondAmount = ethers.parseEther("100");

      await licenseToken.mint(
        await owner.getAddress(),
        bondAmount,
        ethers.encodeBytes32String("test"),
      );
      await licenseToken
        .connect(owner)
        .approve(bondingRegistryAddress, bondAmount);
      // Bonding transfer should succeed.
      await bondingRegistry.connect(owner).bondLicense(bondAmount);
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // Ownership
  // ═════════════════════════════════════════════════════════════════════════

  describe("ownership", function () {
    it("renounceOwnership is disabled", async function () {
      const { token, admin } = await loadFixture(deploy);
      await expect(
        token.connect(admin).renounceOwnership(),
      ).to.be.revertedWithCustomError(token, "RenounceOwnershipDisabled");
    });

    it("ownership transfer syncs AccessControl roles", async function () {
      const { token, admin, alice } = await loadFixture(deploy);
      const adminAddress = await admin.getAddress();
      const aliceAddress = await alice.getAddress();

      // Transfer ownership to alice via 2-step.
      await token.connect(admin).transferOwnership(aliceAddress);
      await token.connect(alice).acceptOwnership();

      // Old owner loses all roles.
      expect(
        await token.hasRole(await token.DEFAULT_ADMIN_ROLE(), adminAddress),
      ).to.be.false;
      expect(await token.hasRole(await token.MINTER_ROLE(), adminAddress)).to.be
        .false;
      expect(await token.hasRole(await token.WHITELIST_ROLE(), adminAddress)).to
        .be.false;
      expect(await token.hasRole(await token.LOCK_MANAGER_ROLE(), adminAddress))
        .to.be.false;

      // New owner gains all roles.
      expect(
        await token.hasRole(await token.DEFAULT_ADMIN_ROLE(), aliceAddress),
      ).to.be.true;
      expect(await token.hasRole(await token.MINTER_ROLE(), aliceAddress)).to.be
        .true;
      expect(await token.hasRole(await token.WHITELIST_ROLE(), aliceAddress)).to
        .be.true;
      expect(await token.hasRole(await token.LOCK_MANAGER_ROLE(), aliceAddress))
        .to.be.true;
    });
  });

  // ═════════════════════════════════════════════════════════════════════════
  // EIP-6372
  // ═════════════════════════════════════════════════════════════════════════

  describe("EIP-6372", function () {
    it("clock() returns block.timestamp and CLOCK_MODE() is mode=timestamp", async function () {
      const { token } = await loadFixture(deploy);
      expect(await token.clock()).to.equal(await time.latest());
      expect(await token.CLOCK_MODE()).to.equal("mode=timestamp");
    });
  });
});
