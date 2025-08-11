import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";
import { expect } from "chai";
import { ethers } from "hardhat";

import { EnclaveToken } from "../../types";

describe("EnclaveToken", function () {
  let enclaveToken: EnclaveToken;
  let owner: SignerWithAddress;
  let addr1: SignerWithAddress;
  let addr2: SignerWithAddress;

  beforeEach(async function () {
    [owner, addr1, addr2] = await ethers.getSigners();

    const EnclaveToken = await ethers.getContractFactory("EnclaveToken");
    enclaveToken = await EnclaveToken.deploy(owner.address);
    await enclaveToken.waitForDeployment();
  });

  describe("Deployment", function () {
    it("Should set the right owner", async function () {
      expect(await enclaveToken.owner()).to.equal(owner.address);
    });

    it("Should have correct name and symbol", async function () {
      expect(await enclaveToken.name()).to.equal("Enclave");
      expect(await enclaveToken.symbol()).to.equal("ENCL");
    });

    it("Should start with zero supply and 18 decimals", async function () {
      expect(await enclaveToken.totalSupply()).to.equal(0);
      expect(await enclaveToken.decimals()).to.equal(18);
    });
  });

  describe("Allocations", function () {
    it("Should allow minting up to cap and revert beyond", async function () {
      const cap = await enclaveToken.TOTAL_SUPPLY();
      const half = cap / 2n;
      await enclaveToken.mintAllocation(addr1.address, half, "Half");
      await enclaveToken.mintAllocation(addr2.address, cap - half, "Rest");
      await expect(
        enclaveToken.mintAllocation(owner.address, 1, "Overflow"),
      ).to.be.revertedWithCustomError(enclaveToken, "ExceedsTotalSupply");
    });
  });

  it("Should allow batch mint allocations", async function () {
    const recipients = [addr1.address, addr2.address];
    const amounts = [ethers.parseEther("1000"), ethers.parseEther("2000")];
    const allocations = ["Investor", "Team"];

    await enclaveToken.batchMintAllocations(recipients, amounts, allocations);

    expect(await enclaveToken.balanceOf(addr1.address)).to.equal(amounts[0]);
    expect(await enclaveToken.balanceOf(addr2.address)).to.equal(amounts[1]);
  });

  it("Should revert if non-owner tries to mint allocation", async function () {
    const amount = ethers.parseEther("1000");

    await expect(
      enclaveToken.connect(addr1).mintAllocation(addr2.address, amount, "Test"),
    ).to.be.revertedWithCustomError(
      enclaveToken,
      "AccessControlUnauthorizedAccount",
    );
  });

  it("Should revert for zero address allocation", async function () {
    const amount = ethers.parseEther("1000");

    await expect(
      enclaveToken.mintAllocation(ethers.ZeroAddress, amount, "Test"),
    ).to.be.revertedWithCustomError(enclaveToken, "ZeroAddress");
  });

  it("Should revert for zero amount allocation", async function () {
    await expect(
      enclaveToken.mintAllocation(addr1.address, 0, "Test"),
    ).to.be.revertedWithCustomError(enclaveToken, "ZeroAmount");
  });

  it("Should revert batch mint with mismatched arrays", async function () {
    const recipients = [addr1.address];
    const amounts = [ethers.parseEther("1000"), ethers.parseEther("2000")];
    const allocations = ["Test"];

    await expect(
      enclaveToken.batchMintAllocations(recipients, amounts, allocations),
    ).to.be.revertedWithCustomError(enclaveToken, "ArrayLengthMismatch");
  });
});

describe("ERC20 Functionality", function () {
  let token: EnclaveToken;
  let owner: SignerWithAddress;
  let addr1: SignerWithAddress;
  let addr2: SignerWithAddress;

  beforeEach(async function () {
    [owner, addr1, addr2] = await ethers.getSigners();
    const EnclaveToken = await ethers.getContractFactory("EnclaveToken");
    token = await EnclaveToken.deploy(owner.address);
    await token.waitForDeployment();
    await token.setTransferRestriction(false);
    await token.mintAllocation(addr1.address, ethers.parseEther("1000"), "Test");
  });

  it("Should allow transfers", async function () {
    const amount = ethers.parseEther("100");
    await expect(token.connect(addr1).transfer(addr2.address, amount))
      .to.emit(token, "Transfer")
      .withArgs(addr1.address, addr2.address, amount);
    expect(await token.balanceOf(addr2.address)).to.equal(amount);
  });

  it("Should allow approvals and transferFrom", async function () {
    const amount = ethers.parseEther("100");
    await token.connect(addr1).approve(addr2.address, amount);
    await token.connect(addr2).transferFrom(addr1.address, addr2.address, amount);
    expect(await token.balanceOf(addr2.address)).to.equal(amount);
  });
});

describe("Governance Features", function () {
  let token: EnclaveToken;
  let owner: SignerWithAddress;
  let addr1: SignerWithAddress;
  let addr2: SignerWithAddress;

  beforeEach(async function () {
    [owner, addr1, addr2] = await ethers.getSigners();
    const EnclaveToken = await ethers.getContractFactory("EnclaveToken");
    token = await EnclaveToken.deploy(owner.address);
    await token.waitForDeployment();
  });

  it("Should support delegation", async function () {
    await token.mintAllocation(addr1.address, ethers.parseEther("1000"), "Gov");
    await token.connect(addr1).delegate(addr2.address);
    expect(await token.getVotes(addr2.address)).to.equal(ethers.parseEther("1000"));
  });

  it("Should support permit functionality", async function () {
    expect(await token.nonces((await ethers.getSigners())[1].address)).to.equal(0);
  });
});

describe("EnclaveToken Transfer Restrictions", function () {
  let token: EnclaveToken;
  let owner: SignerWithAddress;
  let user1: SignerWithAddress;
  let user2: SignerWithAddress;
  let whitelistedContract: SignerWithAddress;

  beforeEach(async function () {
    [owner, user1, user2, whitelistedContract] = await ethers.getSigners();
    console.log(whitelistedContract.address);

    const EnclaveToken = await ethers.getContractFactory("EnclaveToken");
    token = await EnclaveToken.deploy(owner.address);
    await token.waitForDeployment();

    await token.mintAllocation(
      user1.address,
      ethers.parseEther("1000"),
      "Test User 1",
    );
    await token.mintAllocation(
      owner.address,
      ethers.parseEther("1000"),
      "Test Owner",
    );
  });

  describe("Default State", function () {
    it("Should start with transfers restricted", async function () {
      expect(await token.transfersRestricted()).to.be.true;
    });

    it("Should whitelist owner by default", async function () {
      expect(await token.transferWhitelisted(owner.address)).to.be.true;
    });

    it("Should not whitelist regular users", async function () {
      expect(await token.transferWhitelisted(user1.address)).to.be.false;
      expect(await token.transferWhitelisted(user2.address)).to.be.false;
    });
  });

  describe("Transfer Restrictions Enforcement", function () {
    it("Should block transfers from non-whitelisted to non-whitelisted", async function () {
      await expect(
        token.connect(user1).transfer(user2.address, ethers.parseEther("100")),
      ).to.be.revertedWithCustomError(token, "TransferNotAllowed");
    });

    it("Should allow transfers from whitelisted (owner)", async function () {
      await expect(
        token.connect(owner).transfer(user2.address, ethers.parseEther("100")),
      ).to.not.be.reverted;
    });

    it("Should allow transfers to whitelisted", async function () {
      await token.setTransferWhitelist(user2.address, true);

      await expect(
        token.connect(user1).transfer(user2.address, ethers.parseEther("100")),
      ).to.not.be.reverted;
    });

    it("Should allow minting even when restricted", async function () {
      await expect(
        token.mintAllocation(
          user2.address,
          ethers.parseEther("500"),
          "New Mint",
        ),
      ).to.not.be.reverted;

      expect(await token.balanceOf(user2.address)).to.equal(
        ethers.parseEther("500"),
      );
    });

    it("Should allow burning even when restricted", async function () {
      await expect(
        token.mintAllocation(ethers.ZeroAddress, 0, "Burn Test"),
      ).to.be.revertedWithCustomError(token, "ZeroAddress");
    });
  });

  describe("Whitelist Management", function () {
    it("Should allow owner to whitelist addresses", async function () {
      await expect(token.setTransferWhitelist(user1.address, true))
        .to.emit(token, "TransferWhitelistUpdated")
        .withArgs(user1.address, true);

      expect(await token.transferWhitelisted(user1.address)).to.be.true;
    });

    it("Should allow owner to remove from whitelist", async function () {
      await token.setTransferWhitelist(user1.address, true);

      await expect(token.setTransferWhitelist(user1.address, false))
        .to.emit(token, "TransferWhitelistUpdated")
        .withArgs(user1.address, false);

      expect(await token.transferWhitelisted(user1.address)).to.be.false;
    });

    it("Should allow owner to whitelist contracts", async function () {
      await expect(token.whitelistContracts(user1.address, user2.address))
        .to.emit(token, "TransferWhitelistUpdated")
        .withArgs(user1.address, true)
        .and.to.emit(token, "TransferWhitelistUpdated")
        .withArgs(user2.address, true);

      expect(await token.transferWhitelisted(user1.address)).to.be.true;
      expect(await token.transferWhitelisted(user2.address)).to.be.true;
    });

    it("Should not allow non-owner to modify whitelist", async function () {
      await expect(
        token.connect(user1).setTransferWhitelist(user2.address, true),
      ).to.be.revertedWithCustomError(token, "OwnableUnauthorizedAccount");
    });
  });

  describe("Transfer Restriction Toggle", function () {
    it("Should allow owner to disable transfer restrictions", async function () {
      await expect(token.setTransferRestriction(false))
        .to.emit(token, "TransferRestrictionUpdated")
        .withArgs(false);

      expect(await token.transfersRestricted()).to.be.false;

      await expect(
        token.connect(user1).transfer(user2.address, ethers.parseEther("100")),
      ).to.not.be.reverted;
    });

    it("Should allow owner to re-enable transfer restrictions", async function () {
      await token.setTransferRestriction(false);
      await token.setTransferRestriction(true);

      expect(await token.transfersRestricted()).to.be.true;

      await expect(
        token.connect(user1).transfer(user2.address, ethers.parseEther("100")),
      ).to.be.revertedWithCustomError(token, "TransferNotAllowed");
    });
  });

  describe("Integration with OpenZeppelin 5.x _update", function () {
    it("Should work with transferFrom when whitelisted", async function () {
      await token
        .connect(user1)
        .approve(user2.address, ethers.parseEther("100"));

      await token.setTransferWhitelist(user2.address, true);

      await expect(
        token
          .connect(user2)
          .transferFrom(user1.address, user2.address, ethers.parseEther("100")),
      ).to.not.be.reverted;
    });

    it("Should block transferFrom when not whitelisted", async function () {
      await token
        .connect(user1)
        .approve(user2.address, ethers.parseEther("100"));

      await expect(
        token
          .connect(user2)
          .transferFrom(user1.address, user2.address, ethers.parseEther("100")),
      ).to.be.revertedWithCustomError(token, "TransferNotAllowed");
    });
  });
});
