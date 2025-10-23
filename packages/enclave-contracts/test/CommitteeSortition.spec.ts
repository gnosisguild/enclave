// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import { network } from "hardhat";

import BondingRegistryModule from "../ignition/modules/bondingRegistry";
import CommitteeSortitionModule from "../ignition/modules/committeeSortition";
import EnclaveTicketTokenModule from "../ignition/modules/enclaveTicketToken";
import EnclaveTokenModule from "../ignition/modules/enclaveToken";
import MockCiphernodeRegistryEmptyKeyModule from "../ignition/modules/mockCiphernodeRegistryEmptyKey";
import MockStableTokenModule from "../ignition/modules/mockStableToken";
import SlashingManagerModule from "../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  CommitteeSortition__factory as CommitteeSortitionFactory,
  EnclaveTicketToken__factory as EnclaveTicketTokenFactory,
  MockUSDC__factory as MockUSDCFactory,
} from "../types";

const { ethers, networkHelpers, ignition } = await network.connect();
const { loadFixture } = networkHelpers;

describe("CommitteeSortition", function () {
  const SUBMISSION_WINDOW = 300; // 5 minutes
  const TICKET_PRICE = ethers.parseEther("10");
  const E3_ID = 1;
  const THRESHOLD = 3;
  const SEED = 12345;
  const AddressOne = "0x0000000000000000000000000000000000000001";

  async function deployFixture() {
    const [owner, ciphernodeRegistry, node1, node2, node3, node4] =
      await ethers.getSigners();

    const ownerAddress = await owner.getAddress();

    // Deploy token contracts
    const usdcContract = await ignition.deploy(MockStableTokenModule, {
      parameters: {
        MockUSDC: {
          initialSupply: 1000000,
        },
      },
    });

    const enclTokenContract = await ignition.deploy(EnclaveTokenModule, {
      parameters: {
        EnclaveToken: {
          owner: ownerAddress,
        },
      },
    });

    const ticketTokenContract = await ignition.deploy(
      EnclaveTicketTokenModule,
      {
        parameters: {
          EnclaveTicketToken: {
            baseToken: await usdcContract.mockUSDC.getAddress(),
            registry: AddressOne,
            owner: ownerAddress,
          },
        },
      },
    );

    const slashingManagerContract = await ignition.deploy(
      SlashingManagerModule,
      {
        parameters: {
          SlashingManager: {
            admin: ownerAddress,
            bondingRegistry: AddressOne,
          },
        },
      },
    );

    const bondingRegistryContract = await ignition.deploy(
      BondingRegistryModule,
      {
        parameters: {
          BondingRegistry: {
            owner: ownerAddress,
            ticketToken:
              await ticketTokenContract.enclaveTicketToken.getAddress(),
            licenseToken: await enclTokenContract.enclaveToken.getAddress(),
            registry: AddressOne,
            slashedFundsTreasury: ownerAddress,
            ticketPrice: TICKET_PRICE,
            licenseRequiredBond: ethers.parseEther("1000"),
            minTicketBalance: 1,
            exitDelay: 7 * 24 * 60 * 60,
          },
        },
      },
    );

    const committeeSortitionContract = await ignition.deploy(
      CommitteeSortitionModule,
      {
        parameters: {
          CommitteeSortition: {
            bondingRegistry:
              await bondingRegistryContract.bondingRegistry.getAddress(),
            ciphernodeRegistry: ciphernodeRegistry.address,
            submissionWindow: SUBMISSION_WINDOW,
          },
        },
      },
    );

    const bondingRegistry = BondingRegistryFactory.connect(
      await bondingRegistryContract.bondingRegistry.getAddress(),
      owner,
    );
    const committeeSortition = CommitteeSortitionFactory.connect(
      await committeeSortitionContract.committeeSortition.getAddress(),
      owner,
    );

    const usdcToken = MockUSDCFactory.connect(
      await usdcContract.mockUSDC.getAddress(),
      owner,
    );
    const ticketToken = EnclaveTicketTokenFactory.connect(
      await ticketTokenContract.enclaveTicketToken.getAddress(),
      owner,
    );

    // Deploy a mock ciphernode registry for testing
    const mockRegistry = await ignition.deploy(
      MockCiphernodeRegistryEmptyKeyModule,
    );

    // Set up cross-contract dependencies
    await ticketToken.setRegistry(await bondingRegistry.getAddress());
    await bondingRegistry.setRegistry(
      await mockRegistry.mockCiphernodeRegistryEmptyKey.getAddress(),
    );
    await bondingRegistry.setSlashingManager(
      await slashingManagerContract.slashingManager.getAddress(),
    );
    await slashingManagerContract.slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );

    // Set up licensed operators with ticket balances
    const licenseToken = EnclaveTicketTokenFactory.connect(
      await enclTokenContract.enclaveToken.getAddress(),
      owner,
    );
    const licenseAmount = ethers.parseEther("1000"); // Min license bond

    // Whitelist bonding registry for license token transfers
    await enclTokenContract.enclaveToken.whitelistContracts(
      await bondingRegistry.getAddress(),
      ethers.ZeroAddress,
    );

    for (const node of [node1, node2, node3, node4]) {
      const nodeTickets =
        node === node1 ? 5 : node === node2 ? 3 : node === node3 ? 7 : 2;
      const ticketAmount = TICKET_PRICE * BigInt(nodeTickets);

      // Bond license first
      await enclTokenContract.enclaveToken.mintAllocation(
        node.address,
        licenseAmount,
        "Test allocation",
      );
      await licenseToken
        .connect(node)
        .approve(await bondingRegistry.getAddress(), licenseAmount);
      await bondingRegistry.connect(node).bondLicense(licenseAmount);

      // Then register operator
      await bondingRegistry.connect(node).registerOperator();

      // Mint USDC to node and have them add ticket balance through bonding registry
      await usdcToken.mint(node.address, ticketAmount);

      // Node approves ticket token to spend USDC (needed for depositFrom)
      await usdcToken
        .connect(node)
        .approve(await ticketToken.getAddress(), ticketAmount);

      // Node adds ticket balance (this will call ticketToken.depositFrom internally)
      await bondingRegistry.connect(node).addTicketBalance(ticketAmount);
    }

    return {
      committeeSortition,
      bondingRegistry,
      owner,
      ciphernodeRegistry,
      node1,
      node2,
      node3,
      node4,
    };
  }

  describe("Initialization", function () {
    it("Should initialize sortition correctly", async function () {
      const { committeeSortition, ciphernodeRegistry } =
        await loadFixture(deployFixture);
      const requestBlock = await ethers.provider.getBlockNumber();

      await committeeSortition
        .connect(ciphernodeRegistry)
        .initializeSortition(E3_ID, THRESHOLD, SEED, requestBlock);

      const [threshold, seed, reqBlock, deadline, finalized] =
        await committeeSortition.getSortitionInfo(E3_ID);

      expect(threshold).to.equal(THRESHOLD);
      expect(seed).to.equal(SEED);
      expect(reqBlock).to.equal(requestBlock);
      expect(finalized).to.be.false;
      expect(deadline).to.be.gt(0);
    });

    it("Should revert if not called by ciphernode registry", async function () {
      const { committeeSortition, owner } = await loadFixture(deployFixture);
      const requestBlock = await ethers.provider.getBlockNumber();

      await expect(
        committeeSortition
          .connect(owner)
          .initializeSortition(E3_ID, THRESHOLD, SEED, requestBlock),
      ).to.be.revertedWithCustomError(
        committeeSortition,
        "OnlyCiphernodeRegistry",
      );
    });

    it("Should revert if already initialized", async function () {
      const { committeeSortition, ciphernodeRegistry } =
        await loadFixture(deployFixture);
      const requestBlock = await ethers.provider.getBlockNumber();

      await committeeSortition
        .connect(ciphernodeRegistry)
        .initializeSortition(E3_ID, THRESHOLD, SEED, requestBlock);

      await expect(
        committeeSortition
          .connect(ciphernodeRegistry)
          .initializeSortition(E3_ID, THRESHOLD, SEED, requestBlock),
      ).to.be.revertedWithCustomError(
        committeeSortition,
        "CommitteeAlreadyFinalized",
      );
    });
  });

  describe("Ticket Submission", function () {
    async function initializeFixture() {
      const fixture = await deployFixture();
      const requestBlock = await ethers.provider.getBlockNumber();
      await fixture.committeeSortition
        .connect(fixture.ciphernodeRegistry)
        .initializeSortition(E3_ID, THRESHOLD, SEED, requestBlock);
      return { ...fixture, requestBlock };
    }

    it("Should submit ticket successfully", async function () {
      const { committeeSortition, node1 } =
        await loadFixture(initializeFixture);
      const ticketNumber = 1;

      const tx = await committeeSortition
        .connect(node1)
        .submitTicket(E3_ID, ticketNumber);
      await expect(tx).to.emit(committeeSortition, "TicketSubmitted");

      const submission = await committeeSortition.getSubmission(
        E3_ID,
        node1.address,
      );
      expect(submission.exists).to.be.true;
      expect(submission.ticketNumber).to.equal(ticketNumber);
    });

    it("Should track top N nodes correctly", async function () {
      const { committeeSortition, node1, node2, node3, node4 } =
        await loadFixture(initializeFixture);
      // Submit tickets from multiple nodes
      await committeeSortition.connect(node1).submitTicket(E3_ID, 1);
      await committeeSortition.connect(node2).submitTicket(E3_ID, 1);
      await committeeSortition.connect(node3).submitTicket(E3_ID, 1);
      await committeeSortition.connect(node4).submitTicket(E3_ID, 1);

      const topNodes = await committeeSortition.getTopNodes(E3_ID);
      expect(topNodes.length).to.equal(THRESHOLD);
    });

    it("Should revert if ticket number is 0", async function () {
      const { committeeSortition, node1 } =
        await loadFixture(initializeFixture);
      await expect(
        committeeSortition.connect(node1).submitTicket(E3_ID, 0),
      ).to.be.revertedWithCustomError(
        committeeSortition,
        "InvalidTicketNumber",
      );
    });

    it("Should revert if ticket number exceeds available tickets", async function () {
      const { committeeSortition, node1 } =
        await loadFixture(initializeFixture);
      await expect(
        committeeSortition.connect(node1).submitTicket(E3_ID, 100),
      ).to.be.revertedWithCustomError(
        committeeSortition,
        "InvalidTicketNumber",
      );
    });

    it("Should revert if node already submitted", async function () {
      const { committeeSortition, node1 } =
        await loadFixture(initializeFixture);
      await committeeSortition.connect(node1).submitTicket(E3_ID, 1);

      await expect(
        committeeSortition.connect(node1).submitTicket(E3_ID, 2),
      ).to.be.revertedWithCustomError(
        committeeSortition,
        "NodeAlreadySubmitted",
      );
    });

    it("Should revert if node has no tickets", async function () {
      const { committeeSortition } = await loadFixture(initializeFixture);
      // Create a completely fresh wallet
      const nodeWithNoTickets = ethers.Wallet.createRandom().connect(
        ethers.provider,
      );

      // Fund it with ETH for gas but don't set ticket balance
      const [funder] = await ethers.getSigners();
      await funder.sendTransaction({
        to: nodeWithNoTickets.address,
        value: ethers.parseEther("1"),
      });

      // When a node has 0 tickets and tries to submit ticket 1,
      // it will revert with InvalidTicketNumber (since 1 > 0 available tickets)
      await expect(
        committeeSortition.connect(nodeWithNoTickets).submitTicket(E3_ID, 1),
      ).to.be.revertedWithCustomError(
        committeeSortition,
        "InvalidTicketNumber",
      );
    });

    it("Should revert if submission window closed", async function () {
      const { committeeSortition, node1 } =
        await loadFixture(initializeFixture);
      // Fast forward time beyond submission window
      await ethers.provider.send("evm_increaseTime", [SUBMISSION_WINDOW + 1]);
      await ethers.provider.send("evm_mine", []);

      await expect(
        committeeSortition.connect(node1).submitTicket(E3_ID, 1),
      ).to.be.revertedWithCustomError(
        committeeSortition,
        "SubmissionWindowClosed",
      );
    });

    it("Should compute scores correctly", async function () {
      const { committeeSortition, node1 } =
        await loadFixture(initializeFixture);
      const ticketNumber = 1;

      // Compute expected score off-chain
      const expectedScore = await committeeSortition.computeTicketScore(
        node1.address,
        ticketNumber,
        E3_ID,
        SEED,
      );

      await committeeSortition.connect(node1).submitTicket(E3_ID, ticketNumber);

      const submission = await committeeSortition.getSubmission(
        E3_ID,
        node1.address,
      );
      expect(submission.score).to.equal(expectedScore);
    });
  });

  describe("Committee Finalization", function () {
    async function finalizeFixture() {
      const fixture = await deployFixture();
      const requestBlock = await ethers.provider.getBlockNumber();
      await fixture.committeeSortition
        .connect(fixture.ciphernodeRegistry)
        .initializeSortition(E3_ID, THRESHOLD, SEED, requestBlock);

      // Submit tickets from nodes
      await fixture.committeeSortition
        .connect(fixture.node1)
        .submitTicket(E3_ID, 1);
      await fixture.committeeSortition
        .connect(fixture.node2)
        .submitTicket(E3_ID, 1);
      await fixture.committeeSortition
        .connect(fixture.node3)
        .submitTicket(E3_ID, 1);
      return { ...fixture, requestBlock };
    }

    it("Should finalize committee after deadline", async function () {
      const { committeeSortition, owner } = await loadFixture(finalizeFixture);
      // Fast forward time
      await ethers.provider.send("evm_increaseTime", [SUBMISSION_WINDOW + 1]);
      await ethers.provider.send("evm_mine", []);

      const tx = await committeeSortition
        .connect(owner)
        .finalizeCommittee(E3_ID);
      await expect(tx).to.emit(committeeSortition, "CommitteeFinalized");

      const [, , , , finalized] =
        await committeeSortition.getSortitionInfo(E3_ID);
      expect(finalized).to.be.true;
    });

    it("Should revert if finalized before deadline", async function () {
      const { committeeSortition, owner } = await loadFixture(finalizeFixture);
      await expect(
        committeeSortition.connect(owner).finalizeCommittee(E3_ID),
      ).to.be.revertedWithCustomError(
        committeeSortition,
        "SubmissionWindowNotClosed",
      );
    });

    it("Should revert if already finalized", async function () {
      const { committeeSortition, owner } = await loadFixture(finalizeFixture);
      // Fast forward time
      await ethers.provider.send("evm_increaseTime", [SUBMISSION_WINDOW + 1]);
      await ethers.provider.send("evm_mine", []);

      await committeeSortition.connect(owner).finalizeCommittee(E3_ID);

      await expect(
        committeeSortition.connect(owner).finalizeCommittee(E3_ID),
      ).to.be.revertedWithCustomError(
        committeeSortition,
        "CommitteeAlreadyFinalized",
      );
    });

    it("Should return correct committee", async function () {
      const { committeeSortition, owner } = await loadFixture(finalizeFixture);
      // Fast forward time
      await ethers.provider.send("evm_increaseTime", [SUBMISSION_WINDOW + 1]);
      await ethers.provider.send("evm_mine", []);

      const committee = await committeeSortition
        .connect(owner)
        .finalizeCommittee.staticCall(E3_ID);

      expect(committee.length).to.equal(THRESHOLD);
    });

    it("Should prevent submissions after finalization", async function () {
      const { committeeSortition, owner, node4 } =
        await loadFixture(finalizeFixture);
      // Fast forward time
      await ethers.provider.send("evm_increaseTime", [SUBMISSION_WINDOW + 1]);
      await ethers.provider.send("evm_mine", []);

      await committeeSortition.connect(owner).finalizeCommittee(E3_ID);

      // Try to submit - should fail because submission window is closed
      // Note: The contract checks submission window before checking if finalized
      await expect(
        committeeSortition.connect(node4).submitTicket(E3_ID, 1),
      ).to.be.revertedWithCustomError(
        committeeSortition,
        "SubmissionWindowClosed",
      );
    });
  });

  describe("Score Sorting", function () {
    async function scoreSortingFixture() {
      const fixture = await deployFixture();
      const requestBlock = await ethers.provider.getBlockNumber();
      await fixture.committeeSortition
        .connect(fixture.ciphernodeRegistry)
        .initializeSortition(E3_ID, 2, SEED, requestBlock); // Threshold of 2
      return { ...fixture, requestBlock };
    }

    it("Should maintain sorted order (lowest scores first)", async function () {
      const { committeeSortition, node1, node2, node3 } =
        await loadFixture(scoreSortingFixture);
      // Submit tickets
      await committeeSortition.connect(node1).submitTicket(E3_ID, 1);
      await committeeSortition.connect(node2).submitTicket(E3_ID, 1);
      await committeeSortition.connect(node3).submitTicket(E3_ID, 1);

      const topNodes = await committeeSortition.getTopNodes(E3_ID);
      expect(topNodes.length).to.equal(2);

      // Verify scores are in ascending order
      const score1 = (
        await committeeSortition.getSubmission(E3_ID, topNodes[0])
      ).score;
      const score2 = (
        await committeeSortition.getSubmission(E3_ID, topNodes[1])
      ).score;

      expect(score1).to.be.lte(score2);
    });

    it("Should replace worst node when better score arrives", async function () {
      const { committeeSortition, node1, node2, node3 } =
        await loadFixture(scoreSortingFixture);
      await committeeSortition.connect(node1).submitTicket(E3_ID, 1);
      await committeeSortition.connect(node2).submitTicket(E3_ID, 1);

      // const topNodesBefore = await committeeSortition.getTopNodes(E3_ID);

      // Submit from node3 - should replace worst if score is better
      await committeeSortition.connect(node3).submitTicket(E3_ID, 1);

      const topNodesAfter = await committeeSortition.getTopNodes(E3_ID);
      expect(topNodesAfter.length).to.equal(2);
    });
  });
});
