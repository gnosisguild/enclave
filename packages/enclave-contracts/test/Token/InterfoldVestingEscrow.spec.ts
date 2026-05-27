// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";

import { InterfoldVestingEscrow__factory as InterfoldVestingEscrowFactory } from "../../types";
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

describe("InterfoldVestingEscrow", function () {
  async function setup() {
    const signers = await ethers.getSigners();
    const [owner, beneficiary, operator, slasher] = signers;
    const ownerAddress = await owner.getAddress();
    const beneficiaryAddress = await beneficiary.getAddress();
    const operatorAddress = await operator.getAddress();
    const slasherAddress = await slasher.getAddress();

    const sys = await deployEnclaveSystem({
      useMockCiphernodeRegistry: true,
      setupOperators: 0,
      wireSlashingManager: false,
      mintUsdcTo: [],
    });
    const { bondingRegistry, licenseToken } = sys;
    await bondingRegistry.setSlashingManager(slasherAddress);

    const now = BigInt(await time.latest());
    const tge = now + DAY;
    const vestingEscrow = await new InterfoldVestingEscrowFactory(owner).deploy(
      await licenseToken.getAddress(),
      await bondingRegistry.getAddress(),
      tge,
      ownerAddress,
    );
    await vestingEscrow.waitForDeployment();
    const vestingEscrowAddress = await vestingEscrow.getAddress();

    await licenseToken.whitelistContracts(
      await bondingRegistry.getAddress(),
      vestingEscrowAddress,
    );

    await licenseToken.mintAllocation(
      vestingEscrowAddress,
      ethers.parseEther("1000000"),
      "Interfold vesting escrow budget",
    );

    return {
      owner,
      beneficiary,
      operator,
      slasher,
      beneficiaryAddress,
      operatorAddress,
      bondingRegistry,
      licenseToken,
      vestingEscrow,
      tge,
    };
  }

  async function createSchedule(
    overrides: Partial<{
      beneficiary: string;
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
    const fixture = await loadFixture(setup);
    const { vestingEscrow, beneficiaryAddress, tge } = fixture;
    const scheduleId = await vestingEscrow.nextScheduleId();
    const totalAmount = overrides.totalAmount ?? ethers.parseEther("2400");
    await vestingEscrow.createSchedule({
      beneficiary: overrides.beneficiary ?? beneficiaryAddress,
      totalAmount,
      tokenHoldUntil: overrides.tokenHoldUntil ?? tge,
      tokenUnlockStart: overrides.tokenUnlockStart ?? tge,
      tokenUnlockEnd: overrides.tokenUnlockEnd ?? tge + 2n * YEAR,
      serviceStart: overrides.serviceStart ?? 0n,
      serviceCliff: overrides.serviceCliff ?? 0n,
      serviceEnd: overrides.serviceEnd ?? 0n,
      group: overrides.group ?? GROUP_PRE_SEED,
    });
    return { ...fixture, scheduleId, totalAmount };
  }

  it("releases pre-seed/seed/Legion/GG schedules linearly from TGE", async function () {
    const { vestingEscrow, beneficiary, scheduleId, totalAmount, tge } =
      await createSchedule();

    await expect(
      vestingEscrow.connect(beneficiary).claim(scheduleId, totalAmount),
    ).to.be.revertedWithCustomError(vestingEscrow, "NothingClaimable");

    await time.increaseTo(tge + YEAR);

    expect(await vestingEscrow.claimableAmount(scheduleId)).to.equal(
      totalAmount / 2n,
    );

    await expect(
      vestingEscrow.connect(beneficiary).claim(scheduleId, totalAmount / 2n),
    )
      .to.emit(vestingEscrow, "TokensClaimed")
      .withArgs(scheduleId, await beneficiary.getAddress(), totalAmount / 2n);
  });

  it("accumulates Bridge SAFT linear unlock during the holding period", async function () {
    const fixture = await loadFixture(setup);
    const { vestingEscrow, beneficiary, beneficiaryAddress, tge } = fixture;
    const scheduleId = await vestingEscrow.nextScheduleId();
    const totalAmount = ethers.parseEther("2400");

    await vestingEscrow.createSchedule({
      beneficiary: beneficiaryAddress,
      totalAmount,
      tokenHoldUntil: tge + YEAR,
      tokenUnlockStart: tge,
      tokenUnlockEnd: tge + 2n * YEAR,
      serviceStart: 0n,
      serviceCliff: 0n,
      serviceEnd: 0n,
      group: GROUP_BRIDGE,
    });

    await time.increaseTo(tge + YEAR / 2n);
    await expect(
      vestingEscrow.connect(beneficiary).claim(scheduleId, totalAmount),
    ).to.be.revertedWithCustomError(vestingEscrow, "NothingClaimable");

    await time.increaseTo(tge + YEAR);
    expect(await vestingEscrow.claimableAmount(scheduleId)).to.equal(
      totalAmount / 2n,
    );
  });

  it("applies GG team service vesting as a second stricter curve", async function () {
    const fixture = await loadFixture(setup);
    const { vestingEscrow, beneficiary, beneficiaryAddress, tge } = fixture;
    const totalAmount = ethers.parseEther("4800");
    const signing = tge - 180n * DAY;
    const scheduleId = await vestingEscrow.nextScheduleId();

    await vestingEscrow.createSchedule({
      beneficiary: beneficiaryAddress,
      totalAmount,
      tokenHoldUntil: tge,
      tokenUnlockStart: tge,
      tokenUnlockEnd: tge + 2n * YEAR,
      serviceStart: signing,
      serviceCliff: signing + YEAR,
      serviceEnd: signing + 4n * YEAR,
      group: GROUP_TEAM,
    });

    await time.increaseTo(signing + YEAR - DAY);
    await expect(
      vestingEscrow.connect(beneficiary).claim(scheduleId, totalAmount),
    ).to.be.revertedWithCustomError(vestingEscrow, "NothingClaimable");

    await time.increaseTo(signing + YEAR);
    expect(await vestingEscrow.claimableAmount(scheduleId)).to.equal(
      totalAmount / 4n,
    );
  });

  it("lets a beneficiary bond locked tokens and reclaim them through the vesting escrow", async function () {
    const {
      vestingEscrow,
      beneficiary,
      operator,
      operatorAddress,
      bondingRegistry,
      licenseToken,
      scheduleId,
      tge,
    } = await createSchedule();
    const bondAmount = ethers.parseEther("1000");

    await expect(
      vestingEscrow
        .connect(beneficiary)
        .bondLockedTokens(scheduleId, operatorAddress, bondAmount),
    )
      .to.emit(vestingEscrow, "LockedTokensBonded")
      .withArgs(
        scheduleId,
        await beneficiary.getAddress(),
        operatorAddress,
        bondAmount,
      );

    expect(await bondingRegistry.getLicenseBond(operatorAddress)).to.equal(
      bondAmount,
    );
    expect((await vestingEscrow.getSchedule(scheduleId)).bondedAmount).to.equal(
      bondAmount,
    );

    await bondingRegistry.connect(operator).unbondLicense(bondAmount);
    await time.increase(SEVEN_DAYS + 1);

    await expect(bondingRegistry.connect(operator).claimExits(0, bondAmount))
      .to.emit(vestingEscrow, "BondedTokensReturned")
      .withArgs(scheduleId, operatorAddress, bondAmount);

    expect((await vestingEscrow.getSchedule(scheduleId)).bondedAmount).to.equal(
      0n,
    );

    await time.increaseTo(tge + 2n * YEAR);
    await vestingEscrow.connect(beneficiary).claim(scheduleId, bondAmount);
    expect(
      await licenseToken.balanceOf(await beneficiary.getAddress()),
    ).to.equal(bondAmount);
  });

  it("slashes license bond sources in LIFO order across direct and locked bonds", async function () {
    const {
      vestingEscrow,
      beneficiary,
      operator,
      slasher,
      operatorAddress,
      bondingRegistry,
      licenseToken,
      scheduleId,
    } = await createSchedule();

    const lockedBond = ethers.parseEther("400");
    const directBond = ethers.parseEther("200");
    await vestingEscrow
      .connect(beneficiary)
      .bondLockedTokens(scheduleId, operatorAddress, lockedBond);

    await licenseToken.mintAllocation(
      operatorAddress,
      directBond,
      "Operator direct bond",
    );
    await licenseToken
      .connect(operator)
      .approve(await bondingRegistry.getAddress(), directBond);
    await bondingRegistry.connect(operator).bondLicense(directBond);

    await expect(
      bondingRegistry
        .connect(slasher)
        .slashLicenseBond(
          operatorAddress,
          ethers.parseEther("250"),
          ethers.encodeBytes32String("TEST_SLASH"),
        ),
    )
      .to.emit(vestingEscrow, "BondedTokensSlashed")
      .withArgs(scheduleId, operatorAddress, ethers.parseEther("50"));

    expect((await vestingEscrow.getSchedule(scheduleId)).bondedAmount).to.equal(
      ethers.parseEther("350"),
    );
    expect(await bondingRegistry.getLicenseBond(operatorAddress)).to.equal(
      ethers.parseEther("350"),
    );
  });
});
