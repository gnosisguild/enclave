import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";
import { time } from "@nomicfoundation/hardhat-network-helpers";
import { expect } from "chai";
import { ethers } from "hardhat";

import { EnclaveToken, VestingEscrow } from "../../types";

describe("VestingEscrow", function () {
  let enclaveToken: EnclaveToken;
  let vestingEscrow: VestingEscrow;
  let owner: SignerWithAddress;
  let beneficiary: SignerWithAddress;
  let addr2: SignerWithAddress;

  const VESTING_AMOUNT = ethers.parseEther("100000");

  beforeEach(async function () {
    [owner, beneficiary, addr2] = await ethers.getSigners();

    const EnclaveToken = await ethers.getContractFactory("EnclaveToken");
    enclaveToken = await EnclaveToken.deploy(owner.address);
    await enclaveToken.waitForDeployment();

    const VestingEscrow = await ethers.getContractFactory("VestingEscrow");
    vestingEscrow = await VestingEscrow.deploy(
      await enclaveToken.getAddress(),
      owner.address,
    );
    await vestingEscrow.waitForDeployment();

    await enclaveToken.setTransferWhitelist(
      await vestingEscrow.getAddress(),
      true,
    );
  });

  describe("Deployment", function () {
    it("Should set the right token and owner", async function () {
      expect(await vestingEscrow.ENCL_TOKEN()).to.equal(
        await enclaveToken.getAddress(),
      );
      expect(await vestingEscrow.owner()).to.equal(owner.address);
    });

    it("Should initialize with zero escrow amounts", async function () {
      expect(await vestingEscrow.totalEscrowed()).to.equal(0);
      expect(await vestingEscrow.totalClaimed()).to.equal(0);
    });
  });

  describe("Creating Vesting Streams", function () {
    it("Should create a vesting stream", async function () {
      const startTime = (await time.latest()) + 3600;
      const cliffDuration = 86400;
      const vestingDuration = 86400 * 30;

      await enclaveToken.mintAllocation(
        owner.address,
        VESTING_AMOUNT,
        "Vesting",
      );
      await enclaveToken.approve(
        await vestingEscrow.getAddress(),
        VESTING_AMOUNT,
      );

      await expect(
        vestingEscrow.createVestingStream(
          beneficiary.address,
          VESTING_AMOUNT,
          startTime,
          cliffDuration,
          vestingDuration,
        ),
      )
        .to.emit(vestingEscrow, "VestingStreamCreated")
        .withArgs(
          beneficiary.address,
          VESTING_AMOUNT,
          startTime,
          cliffDuration,
          vestingDuration,
        );

      expect(await vestingEscrow.totalEscrowed()).to.equal(VESTING_AMOUNT);
      expect(
        await enclaveToken.balanceOf(await vestingEscrow.getAddress()),
      ).to.equal(VESTING_AMOUNT);

      const stream = await vestingEscrow.vestingStreams(beneficiary.address);
      expect(stream.totalAmount).to.equal(VESTING_AMOUNT);
      expect(stream.startTime).to.equal(startTime);
      expect(stream.cliffDuration).to.equal(cliffDuration);
      expect(stream.vestingDuration).to.equal(vestingDuration);
      expect(stream.claimed).to.equal(0);
      expect(stream.revoked).to.be.false;
    });

    it("Should create batch vesting streams", async function () {
      const beneficiaries = [beneficiary.address, addr2.address];
      const amounts = [VESTING_AMOUNT, ethers.parseEther("50000")];
      const startTime = await time.latest();
      const startTimes = [startTime, startTime];
      const cliffDurations = [86400, 0];
      const vestingDurations = [86400 * 30, 86400 * 60];

      const totalAmount = amounts[0] + amounts[1];
      await enclaveToken.mintAllocation(owner.address, totalAmount, "Batch");
      await enclaveToken.approve(await vestingEscrow.getAddress(), totalAmount);

      await vestingEscrow.batchCreateVestingStreams(
        beneficiaries,
        amounts,
        startTimes,
        cliffDurations,
        vestingDurations,
      );

      expect(await vestingEscrow.totalEscrowed()).to.equal(totalAmount);
    });

    it("Should revert if non-owner tries to create stream", async function () {
      const startTime = await time.latest();

      await expect(
        vestingEscrow
          .connect(beneficiary)
          .createVestingStream(
            beneficiary.address,
            VESTING_AMOUNT,
            startTime,
            0,
            86400,
          ),
      ).to.be.revertedWithCustomError(
        vestingEscrow,
        "OwnableUnauthorizedAccount",
      );
    });

    it("Should revert for invalid parameters", async function () {
      const startTime = await time.latest();

      await expect(
        vestingEscrow.createVestingStream(
          ethers.ZeroAddress,
          VESTING_AMOUNT,
          startTime,
          0,
          86400,
        ),
      ).to.be.revertedWithCustomError(vestingEscrow, "ZeroAddress");

      await expect(
        vestingEscrow.createVestingStream(
          beneficiary.address,
          0,
          startTime,
          0,
          86400,
        ),
      ).to.be.revertedWithCustomError(vestingEscrow, "ZeroAmount");

      await expect(
        vestingEscrow.createVestingStream(
          beneficiary.address,
          VESTING_AMOUNT,
          startTime,
          0,
          0,
        ),
      ).to.be.revertedWithCustomError(vestingEscrow, "ZeroVestingDuration");

      await expect(
        vestingEscrow.createVestingStream(
          beneficiary.address,
          VESTING_AMOUNT,
          startTime,
          86400,
          3600,
        ),
      ).to.be.revertedWithCustomError(vestingEscrow, "CliffExceedsVesting");
    });

    it("Should revert if stream already exists", async function () {
      const startTime = await time.latest();

      await enclaveToken.mintAllocation(owner.address, VESTING_AMOUNT * 2n, "Dup");
      await enclaveToken.approve(
        await vestingEscrow.getAddress(),
        VESTING_AMOUNT * 2n,
      );

      await vestingEscrow.createVestingStream(
        beneficiary.address,
        VESTING_AMOUNT,
        startTime,
        0,
        86400,
      );

      await expect(
        vestingEscrow.createVestingStream(
          beneficiary.address,
          VESTING_AMOUNT,
          startTime,
          0,
          86400,
        ),
      ).to.be.revertedWithCustomError(vestingEscrow, "StreamAlreadyExists");
    });
  });

  describe("Claiming Vested Tokens", function () {
    let startTime: number;
    const cliffDuration = 86400;
    const vestingDuration = 86400 * 30;

    beforeEach(async function () {
      startTime = await time.latest();

      await enclaveToken.mintAllocation(owner.address, VESTING_AMOUNT, "Claim");
      await enclaveToken.approve(
        await vestingEscrow.getAddress(),
        VESTING_AMOUNT,
      );
      await vestingEscrow.createVestingStream(
        beneficiary.address,
        VESTING_AMOUNT,
        startTime,
        cliffDuration,
        vestingDuration,
      );
    });

    it("Should return zero claimable before cliff", async function () {
      expect(
        await vestingEscrow.getClaimableAmount(beneficiary.address),
      ).to.equal(0);
    });

    it("Should not allow claiming before cliff", async function () {
      await expect(
        vestingEscrow.connect(beneficiary).claim(),
      ).to.be.revertedWithCustomError(vestingEscrow, "NoTokensToClaim");
    });

    it("Should allow claiming after cliff", async function () {
      await time.increaseTo(startTime + cliffDuration + 1);

      const claimable = await vestingEscrow.getClaimableAmount(
        beneficiary.address,
      );
      expect(claimable).to.be.gt(0);

      await expect(vestingEscrow.connect(beneficiary).claim()).to.emit(
        vestingEscrow,
        "TokensClaimed",
      );

      const balance = await enclaveToken.balanceOf(beneficiary.address);
      const streamAfter = await vestingEscrow.vestingStreams(
        beneficiary.address,
      );
      expect(balance).to.equal(streamAfter.claimed);
    });

    it("Should calculate vesting correctly over time", async function () {
      const halfwayTime = startTime + vestingDuration / 2;
      await time.increaseTo(halfwayTime);

      const claimable = await vestingEscrow.getClaimableAmount(
        beneficiary.address,
      );
      const expectedHalfway = VESTING_AMOUNT / 2n;

      expect(claimable).to.be.closeTo(
        expectedHalfway,
        ethers.parseEther("1000"),
      );
    });

    it("Should allow claiming full amount after vesting period", async function () {
      await time.increaseTo(startTime + vestingDuration + 1);

      const claimable = await vestingEscrow.getClaimableAmount(
        beneficiary.address,
      );
      expect(claimable).to.equal(VESTING_AMOUNT);

      await vestingEscrow.connect(beneficiary).claim();
      expect(await enclaveToken.balanceOf(beneficiary.address)).to.equal(
        VESTING_AMOUNT,
      );
    });

    it("Should prevent double claiming", async function () {
      await time.increaseTo(startTime + vestingDuration + 1);

      await vestingEscrow.connect(beneficiary).claim();

      expect(
        await vestingEscrow.getClaimableAmount(beneficiary.address),
      ).to.equal(0);
    });

    it("Should revert claiming for non-existent stream", async function () {
      await expect(
        vestingEscrow.connect(addr2).claim(),
      ).to.be.revertedWithCustomError(vestingEscrow, "NoVestingStream");
    });
  });

  describe("Stream Revocation", function () {
    let startTime: number;
    const vestingDuration = 86400 * 30;

    beforeEach(async function () {
      startTime = await time.latest();

      await enclaveToken.mintAllocation(owner.address, VESTING_AMOUNT, "Revoke");
      await enclaveToken.approve(
        await vestingEscrow.getAddress(),
        VESTING_AMOUNT,
      );
      await vestingEscrow.createVestingStream(
        beneficiary.address,
        VESTING_AMOUNT,
        startTime,
        0,
        vestingDuration,
      );
    });

    it("Should allow owner to revoke stream", async function () {
      const targetTs = startTime + vestingDuration / 2;
      await time.setNextBlockTimestamp(targetTs);

      const ownerBefore = await enclaveToken.balanceOf(owner.address);
      const beneBefore = await enclaveToken.balanceOf(beneficiary.address);

      const tx = await vestingEscrow.revokeVestingStream(beneficiary.address);
      const rcpt = await tx.wait();

      const parsed = rcpt?.logs
        .map((l) => {
          try {
            return vestingEscrow.interface.parseLog(l);
          } catch {
            return null;
          }
        })
        .filter(Boolean) as Array<ReturnType<typeof vestingEscrow.interface.parseLog>>;

      const claimedEvt = parsed.find((p) => p!.name === "TokensClaimed");
      const revokedEvt = parsed.find((p) => p!.name === "VestingStreamRevoked");

      const claimed = (claimedEvt?.args?.[1] as bigint) ?? 0n;
      const unvested = (revokedEvt?.args?.[1] as bigint) ?? 0n;

      const ownerAfter = await enclaveToken.balanceOf(owner.address);
      const beneAfter = await enclaveToken.balanceOf(beneficiary.address);

      expect(beneAfter - beneBefore).to.equal(claimed);
      expect(ownerAfter - ownerBefore).to.equal(unvested);
      expect(claimed + unvested).to.equal(VESTING_AMOUNT);

      const stream = await vestingEscrow.vestingStreams(beneficiary.address);
      expect(stream.revoked).to.be.true;
    });

    it("Should revert if non-owner tries to revoke", async function () {
      await expect(
        vestingEscrow
          .connect(beneficiary)
          .revokeVestingStream(beneficiary.address),
      ).to.be.revertedWithCustomError(
        vestingEscrow,
        "OwnableUnauthorizedAccount",
      );
    });

    it("Should revert claiming from revoked stream", async function () {
      await vestingEscrow.revokeVestingStream(beneficiary.address);

      await expect(
        vestingEscrow.connect(beneficiary).claim(),
      ).to.be.revertedWithCustomError(vestingEscrow, "StreamRevoked");
    });
  });

  describe("View Functions", function () {
    it("Should return correct remaining vesting time", async function () {
      const startTime = await time.latest();
      const vestingDuration = 86400 * 30;

      await enclaveToken.mintAllocation(owner.address, VESTING_AMOUNT, "View");
      await enclaveToken.approve(
        await vestingEscrow.getAddress(),
        VESTING_AMOUNT,
      );
      await vestingEscrow.createVestingStream(
        beneficiary.address,
        VESTING_AMOUNT,
        startTime,
        0,
        vestingDuration,
      );

      const remainingAtStart = await vestingEscrow.getRemainingVestingTime(
        beneficiary.address,
      );
      expect(remainingAtStart).to.be.closeTo(vestingDuration, 5);

      await time.increaseTo(startTime + vestingDuration / 2);
      const remaining = await vestingEscrow.getRemainingVestingTime(
        beneficiary.address,
      );
      expect(remaining).to.be.closeTo(vestingDuration / 2, 10);

      await time.increaseTo(startTime + vestingDuration + 1);
      expect(
        await vestingEscrow.getRemainingVestingTime(beneficiary.address),
      ).to.equal(0);
    });
  });
});
