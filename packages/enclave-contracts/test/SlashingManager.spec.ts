import { expect } from "chai";
import { network } from "hardhat";

import BondingRegistryModule from "../ignition/modules/bondingRegistry";
import EnclaveTicketTokenModule from "../ignition/modules/enclaveTicketToken";
import EnclaveTokenModule from "../ignition/modules/enclaveToken";
import MockSlashingVerifierModule from "../ignition/modules/mockSlashingVerifier";
import MockStableTokenModule from "../ignition/modules/mockStableToken";
import SlashingManagerModule from "../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  EnclaveTicketToken__factory as EnclaveTicketTokenFactory,
  EnclaveToken__factory as EnclaveTokenFactory,
  MockSlashingVerifier__factory as MockSlashingVerifierFactory,
  MockUSDC__factory as MockUSDCFactory,
  SlashingManager__factory as SlashingManagerFactory,
} from "../types";
import type { SlashingManager } from "../types/contracts/slashing/SlashingManager";
import type { MockSlashingVerifier } from "../types/contracts/test/MockSlashingVerifier";

const { ethers, networkHelpers, ignition } = await network.connect();
const { loadFixture, time } = networkHelpers;

describe("SlashingManager", function () {
  const REASON_MISBEHAVIOR = ethers.encodeBytes32String("misbehavior");
  const REASON_INACTIVITY = ethers.encodeBytes32String("inactivity");
  const REASON_DOUBLE_SIGN = ethers.encodeBytes32String("doubleSign");

  const SLASHER_ROLE = ethers.keccak256(ethers.toUtf8Bytes("SLASHER_ROLE"));
  const VERIFIER_ROLE = ethers.keccak256(ethers.toUtf8Bytes("VERIFIER_ROLE"));
  const GOVERNANCE_ROLE = ethers.keccak256(
    ethers.toUtf8Bytes("GOVERNANCE_ROLE"),
  );
  const DEFAULT_ADMIN_ROLE = ethers.ZeroHash;

  async function setupPolicies(
    slashingManager: SlashingManager,
    mockVerifier: MockSlashingVerifier,
  ) {
    const proofPolicy = {
      ticketPenalty: ethers.parseUnits("50", 6),
      licensePenalty: ethers.parseEther("100"),
      requiresProof: true,
      proofVerifier: await mockVerifier.getAddress(),
      banNode: false,
      appealWindow: 0,
      enabled: true,
    };

    const evidencePolicy = {
      ticketPenalty: ethers.parseUnits("20", 6),
      licensePenalty: ethers.parseEther("50"),
      requiresProof: false,
      proofVerifier: ethers.ZeroAddress,
      banNode: false,
      appealWindow: 7 * 24 * 60 * 60,
      enabled: true,
    };

    const banPolicy = {
      ticketPenalty: ethers.parseUnits("100", 6),
      licensePenalty: ethers.parseEther("500"),
      requiresProof: true,
      proofVerifier: await mockVerifier.getAddress(),
      banNode: true,
      appealWindow: 0,
      enabled: true,
    };

    await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);
    await slashingManager.setSlashPolicy(REASON_INACTIVITY, evidencePolicy);
    await slashingManager.setSlashPolicy(REASON_DOUBLE_SIGN, banPolicy);
  }

  async function setup() {
    const [owner, slasher, verifier, operator, notTheOwner] =
      await ethers.getSigners();
    const ownerAddress = await owner.getAddress();
    const operatorAddress = await operator.getAddress();

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
            underlyingUSDC: await usdcContract.mockUSDC.getAddress(),
            registry: ownerAddress,
            owner: ownerAddress,
          },
        },
      },
    );

    const mockVerifierContract = await ignition.deploy(
      MockSlashingVerifierModule,
    );

    const slashingManagerContract = await ignition.deploy(
      SlashingManagerModule,
      {
        parameters: {
          SlashingManager: {
            admin: ownerAddress,
            bondingRegistry: ownerAddress,
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
            registry: ethers.ZeroAddress,
            slashedFundsTreasury: ownerAddress,
            ticketPrice: ethers.parseUnits("10", 6),
            licenseRequiredBond: ethers.parseEther("1000"),
            minTicketBalance: 5,
            exitDelay: 7 * 24 * 60 * 60,
          },
        },
      },
    );

    const usdcToken = MockUSDCFactory.connect(
      await usdcContract.mockUSDC.getAddress(),
      owner,
    );
    const enclaveToken = EnclaveTokenFactory.connect(
      await enclTokenContract.enclaveToken.getAddress(),
      owner,
    );
    const ticketToken = EnclaveTicketTokenFactory.connect(
      await ticketTokenContract.enclaveTicketToken.getAddress(),
      owner,
    );
    const mockVerifier = MockSlashingVerifierFactory.connect(
      await mockVerifierContract.mockSlashingVerifier.getAddress(),
      owner,
    );
    const slashingManager = SlashingManagerFactory.connect(
      await slashingManagerContract.slashingManager.getAddress(),
      owner,
    );
    const bondingRegistry = BondingRegistryFactory.connect(
      await bondingRegistryContract.bondingRegistry.getAddress(),
      owner,
    );

    await ticketToken.setRegistry(await bondingRegistry.getAddress());
    await slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );
    await bondingRegistry.setSlashingManager(
      await slashingManager.getAddress(),
    );

    await enclaveToken.setTransferRestriction(false);

    await enclaveToken.mintAllocation(
      operatorAddress,
      ethers.parseEther("2000"),
      "Test allocation",
    );

    await slashingManager.addSlasher(await slasher.getAddress());
    await slashingManager.addVerifier(await verifier.getAddress());

    return {
      owner,
      slasher,
      verifier,
      operator,
      operatorAddress,
      notTheOwner,
      slashingManager,
      bondingRegistry,
      enclaveToken,
      ticketToken,
      usdcToken,
      mockVerifier,
    };
  }

  describe("constructor / initialization", function () {
    it("should set the admin role correctly", async function () {
      const { slashingManager, owner } = await loadFixture(setup);

      expect(
        await slashingManager.hasRole(
          DEFAULT_ADMIN_ROLE,
          await owner.getAddress(),
        ),
      ).to.be.true;
      expect(
        await slashingManager.hasRole(
          GOVERNANCE_ROLE,
          await owner.getAddress(),
        ),
      ).to.be.true;
    });

    it("should set the bonding registry correctly", async function () {
      const { slashingManager, bondingRegistry } = await loadFixture(setup);

      expect(await slashingManager.bondingRegistry()).to.equal(
        await bondingRegistry.getAddress(),
      );
    });

    it("should revert if admin is zero address", async function () {
      await expect(
        ignition.deploy(SlashingManagerModule, {
          parameters: {
            SlashingManager: {
              admin: ethers.ZeroAddress,
              bondingRegistry: ethers.ZeroAddress,
            },
          },
        }),
      ).to.be.rejected;
    });
  });

  describe("setSlashPolicy()", function () {
    it("should set a valid slash policy", async function () {
      const { slashingManager, mockVerifier } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: await mockVerifier.getAddress(),
        banNode: false,
        appealWindow: 0,
        enabled: true,
      };

      await expect(slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, policy))
        .to.emit(slashingManager, "SlashPolicyUpdated")
        .withArgs(REASON_MISBEHAVIOR, Object.values(policy));

      const storedPolicy =
        await slashingManager.getSlashPolicy(REASON_MISBEHAVIOR);
      expect(storedPolicy.ticketPenalty).to.equal(policy.ticketPenalty);
      expect(storedPolicy.licensePenalty).to.equal(policy.licensePenalty);
      expect(storedPolicy.requiresProof).to.equal(policy.requiresProof);
      expect(storedPolicy.enabled).to.equal(policy.enabled);
    });

    it("should set a policy without proof requirement", async function () {
      const { slashingManager } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("20", 6),
        licensePenalty: ethers.parseEther("50"),
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 7 * 24 * 60 * 60,
        enabled: true,
      };

      await expect(slashingManager.setSlashPolicy(REASON_INACTIVITY, policy))
        .to.emit(slashingManager, "SlashPolicyUpdated")
        .withArgs(REASON_INACTIVITY, Object.values(policy));
    });

    it("should revert if caller is not governance", async function () {
      const { slashingManager, notTheOwner } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 7 * 24 * 60 * 60,
        enabled: true,
      };

      await expect(
        slashingManager
          .connect(notTheOwner)
          .setSlashPolicy(REASON_MISBEHAVIOR, policy),
      ).to.be.revertedWithCustomError(
        slashingManager,
        "AccessControlUnauthorizedAccount",
      );
    });

    it("should revert if reason is zero", async function () {
      const { slashingManager } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 7 * 24 * 60 * 60,
        enabled: true,
      };

      await expect(
        slashingManager.setSlashPolicy(ethers.ZeroHash, policy),
      ).to.be.revertedWithCustomError(slashingManager, "InvalidPolicy");
    });

    it("should revert if policy is disabled", async function () {
      const { slashingManager } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 7 * 24 * 60 * 60,
        enabled: false,
      };

      await expect(
        slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, policy),
      ).to.be.revertedWithCustomError(slashingManager, "InvalidPolicy");
    });

    it("should revert if no penalties are set", async function () {
      const { slashingManager } = await loadFixture(setup);

      const policy = {
        ticketPenalty: 0,
        licensePenalty: 0,
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 7 * 24 * 60 * 60,
        enabled: true,
      };

      await expect(
        slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, policy),
      ).to.be.revertedWithCustomError(slashingManager, "InvalidPolicy");
    });

    it("should revert if proof required but no verifier set", async function () {
      const { slashingManager } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 0,
        enabled: true,
      };

      await expect(
        slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, policy),
      ).to.be.revertedWithCustomError(slashingManager, "VerifierNotSet");
    });

    it("should revert if proof required but appeal window set", async function () {
      const { slashingManager, mockVerifier } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: await mockVerifier.getAddress(),
        banNode: false,
        appealWindow: 7 * 24 * 60 * 60,
        enabled: true,
      };

      await expect(
        slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, policy),
      ).to.be.revertedWithCustomError(slashingManager, "InvalidPolicy");
    });

    it("should revert if no proof required but no appeal window", async function () {
      const { slashingManager } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 0,
        enabled: true,
      };

      await expect(
        slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, policy),
      ).to.be.revertedWithCustomError(slashingManager, "InvalidPolicy");
    });
  });

  describe("role management", function () {
    it("should add and remove slasher role", async function () {
      const { slashingManager, notTheOwner } = await loadFixture(setup);

      await slashingManager.addSlasher(await notTheOwner.getAddress());
      expect(
        await slashingManager.hasRole(
          SLASHER_ROLE,
          await notTheOwner.getAddress(),
        ),
      ).to.be.true;

      await slashingManager.removeSlasher(await notTheOwner.getAddress());
      expect(
        await slashingManager.hasRole(
          SLASHER_ROLE,
          await notTheOwner.getAddress(),
        ),
      ).to.be.false;
    });

    it("should add and remove verifier role", async function () {
      const { slashingManager, notTheOwner } = await loadFixture(setup);

      await slashingManager.addVerifier(await notTheOwner.getAddress());
      expect(
        await slashingManager.hasRole(
          VERIFIER_ROLE,
          await notTheOwner.getAddress(),
        ),
      ).to.be.true;

      await slashingManager.removeVerifier(await notTheOwner.getAddress());
      expect(
        await slashingManager.hasRole(
          VERIFIER_ROLE,
          await notTheOwner.getAddress(),
        ),
      ).to.be.false;
    });

    it("should revert if non-admin tries to add slasher", async function () {
      const { slashingManager, notTheOwner } = await loadFixture(setup);

      await expect(
        slashingManager
          .connect(notTheOwner)
          .addSlasher(await notTheOwner.getAddress()),
      ).to.be.revert(ethers);
    });

    it("should revert if zero address is added as slasher", async function () {
      const { slashingManager } = await loadFixture(setup);

      await expect(
        slashingManager.addSlasher(ethers.ZeroAddress),
      ).to.be.revertedWithCustomError(slashingManager, "ZeroAddress");
    });
  });

  describe("proposeSlash()", function () {
    it("should propose slash with proof", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      const proofPolicy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: await mockVerifier.getAddress(),
        banNode: false,
        appealWindow: 0,
        enabled: true,
      };
      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);

      const proof = ethers.toUtf8Bytes("Valid proof data");
      const currentTime = await time.latest();

      await expect(
        slashingManager
          .connect(slasher)
          .proposeSlash(operatorAddress, REASON_MISBEHAVIOR, proof),
      )
        .to.emit(slashingManager, "SlashProposed")
        .withArgs(
          0,
          operatorAddress,
          REASON_MISBEHAVIOR,
          ethers.parseUnits("50", 6),
          ethers.parseEther("100"),
          currentTime + 1,
          await slasher.getAddress(),
        );

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.operator).to.equal(operatorAddress);
      expect(proposal.reason).to.equal(REASON_MISBEHAVIOR);
      expect(proposal.proofVerified).to.be.true;
      expect(proposal.proposer).to.equal(await slasher.getAddress());
    });

    it("should propose slash without proof (evidence-based)", async function () {
      const { slashingManager, slasher, operatorAddress } =
        await loadFixture(setup);

      const evidencePolicy = {
        ticketPenalty: ethers.parseUnits("20", 6),
        licensePenalty: ethers.parseEther("50"),
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 7 * 24 * 60 * 60,
        enabled: true,
      };
      await slashingManager.setSlashPolicy(REASON_INACTIVITY, evidencePolicy);

      const proof = ethers.toUtf8Bytes("");
      const currentTime = await time.latest();
      const appealWindow = 7 * 24 * 60 * 60;

      await expect(
        slashingManager
          .connect(slasher)
          .proposeSlash(operatorAddress, REASON_INACTIVITY, proof),
      )
        .to.emit(slashingManager, "SlashProposed")
        .withArgs(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.parseUnits("20", 6),
          ethers.parseEther("50"),
          currentTime + appealWindow + 1,
          await slasher.getAddress(),
        );

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.proofVerified).to.be.false;
      expect(proposal.executableAt).to.be.greaterThan(
        currentTime + appealWindow,
      );
    });

    it("should revert if caller is not slasher", async function () {
      const { slashingManager, notTheOwner, operatorAddress } =
        await loadFixture(setup);

      const proof = ethers.toUtf8Bytes("Some proof");

      await expect(
        slashingManager
          .connect(notTheOwner)
          .proposeSlash(operatorAddress, REASON_MISBEHAVIOR, proof),
      ).to.be.revertedWithCustomError(slashingManager, "Unauthorized");
    });

    it("should revert if operator is zero address", async function () {
      const { slashingManager, slasher } = await loadFixture(setup);

      const proof = ethers.toUtf8Bytes("Some proof");

      await expect(
        slashingManager
          .connect(slasher)
          .proposeSlash(ethers.ZeroAddress, REASON_MISBEHAVIOR, proof),
      ).to.be.revertedWithCustomError(slashingManager, "ZeroAddress");
    });

    it("should revert if slash reason is disabled", async function () {
      const { slashingManager, slasher, operatorAddress } =
        await loadFixture(setup);

      const proof = ethers.toUtf8Bytes("Some proof");

      await expect(
        slashingManager
          .connect(slasher)
          .proposeSlash(operatorAddress, REASON_DOUBLE_SIGN, proof),
      ).to.be.revertedWithCustomError(slashingManager, "SlashReasonDisabled");
    });

    it("should revert if proof required but not provided", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      const proofPolicy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: await mockVerifier.getAddress(),
        banNode: false,
        appealWindow: 0,
        enabled: true,
      };
      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);

      const emptyProof = ethers.toUtf8Bytes("");

      await expect(
        slashingManager
          .connect(slasher)
          .proposeSlash(operatorAddress, REASON_MISBEHAVIOR, emptyProof),
      ).to.be.revertedWithCustomError(slashingManager, "ProofRequired");
    });

    it("should increment totalProposals", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      const proofPolicy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: await mockVerifier.getAddress(),
        banNode: false,
        appealWindow: 0,
        enabled: true,
      };
      const evidencePolicy = {
        ticketPenalty: ethers.parseUnits("20", 6),
        licensePenalty: ethers.parseEther("50"),
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 7 * 24 * 60 * 60,
        enabled: true,
      };
      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);
      await slashingManager.setSlashPolicy(REASON_INACTIVITY, evidencePolicy);

      expect(await slashingManager.totalProposals()).to.equal(0);

      const proof = ethers.toUtf8Bytes("Valid proof");
      await slashingManager
        .connect(slasher)
        .proposeSlash(operatorAddress, REASON_MISBEHAVIOR, proof);

      expect(await slashingManager.totalProposals()).to.equal(1);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );

      expect(await slashingManager.totalProposals()).to.equal(2);
    });
  });

  describe("executeSlash()", function () {
    it("should execute slash with proof immediately", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      const proofPolicy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: await mockVerifier.getAddress(),
        banNode: false,
        appealWindow: 0,
        enabled: true,
      };
      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);

      const proof = ethers.toUtf8Bytes("Valid proof");
      await slashingManager
        .connect(slasher)
        .proposeSlash(operatorAddress, REASON_MISBEHAVIOR, proof);

      await expect(slashingManager.connect(slasher).executeSlash(0))
        .to.emit(slashingManager, "SlashExecuted")
        .withArgs(
          0,
          operatorAddress,
          REASON_MISBEHAVIOR,
          ethers.parseUnits("50", 6),
          ethers.parseEther("100"),
          true,
          true,
        );

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.executedTicket).to.be.true;
      expect(proposal.executedLicense).to.be.true;
    });

    it("should execute slash after appeal window expires", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );

      await expect(
        slashingManager.connect(slasher).executeSlash(0),
      ).to.be.revertedWithCustomError(slashingManager, "AppealWindowActive");

      await time.increase(7 * 24 * 60 * 60 + 1);

      await expect(slashingManager.connect(slasher).executeSlash(0)).to.emit(
        slashingManager,
        "SlashExecuted",
      );
    });

    it("should ban node when policy requires it", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      const proof = ethers.toUtf8Bytes("Serious violation proof");
      await slashingManager
        .connect(slasher)
        .proposeSlash(operatorAddress, REASON_DOUBLE_SIGN, proof);

      expect(await slashingManager.isBanned(operatorAddress)).to.be.false;

      await expect(slashingManager.connect(slasher).executeSlash(0))
        .to.emit(slashingManager, "NodeBanned")
        .withArgs(
          operatorAddress,
          REASON_DOUBLE_SIGN,
          await slashingManager.getAddress(),
        );

      expect(await slashingManager.isBanned(operatorAddress)).to.be.true;
    });

    it("should revert if proposal doesn't exist", async function () {
      const { slashingManager, slasher } = await loadFixture(setup);

      await expect(
        slashingManager.connect(slasher).executeSlash(999),
      ).to.be.revertedWithCustomError(slashingManager, "InvalidProposal");
    });

    it("should revert if already executed", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      const proof = ethers.toUtf8Bytes("Valid proof");
      await slashingManager
        .connect(slasher)
        .proposeSlash(operatorAddress, REASON_MISBEHAVIOR, proof);
      await slashingManager.connect(slasher).executeSlash(0);

      await expect(
        slashingManager.connect(slasher).executeSlash(0),
      ).to.be.revertedWithCustomError(slashingManager, "AlreadyExecuted");
    });

    it("should revert if caller is not slasher", async function () {
      const {
        slashingManager,
        slasher,
        notTheOwner,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      const proof = ethers.toUtf8Bytes("Valid proof");
      await slashingManager
        .connect(slasher)
        .proposeSlash(operatorAddress, REASON_MISBEHAVIOR, proof);

      await expect(
        slashingManager.connect(notTheOwner).executeSlash(0),
      ).to.be.revertedWithCustomError(slashingManager, "Unauthorized");
    });
  });

  describe("appeal system", function () {
    it("should allow operator to file appeal", async function () {
      const {
        slashingManager,
        slasher,
        operator,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );

      const evidence = "I was not inactive, here's the proof...";

      await expect(slashingManager.connect(operator).fileAppeal(0, evidence))
        .to.emit(slashingManager, "AppealFiled")
        .withArgs(0, operatorAddress, REASON_INACTIVITY, evidence);

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.appealed).to.be.true;
    });

    it("should revert if non-operator tries to appeal", async function () {
      const {
        slashingManager,
        slasher,
        notTheOwner,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );

      await expect(
        slashingManager.connect(notTheOwner).fileAppeal(0, "Not my appeal"),
      ).to.be.revertedWithCustomError(slashingManager, "Unauthorized");
    });

    it("should revert if appeal window expired", async function () {
      const {
        slashingManager,
        slasher,
        operator,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );

      await time.increase(7 * 24 * 60 * 60 + 1);

      await expect(
        slashingManager.connect(operator).fileAppeal(0, "Too late"),
      ).to.be.revertedWithCustomError(slashingManager, "AppealWindowExpired");
    });

    it("should revert if already appealed", async function () {
      const {
        slashingManager,
        slasher,
        operator,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );

      await slashingManager.connect(operator).fileAppeal(0, "First appeal");

      await expect(
        slashingManager.connect(operator).fileAppeal(0, "Second appeal"),
      ).to.be.revertedWithCustomError(slashingManager, "AlreadyAppealed");
    });

    it("should allow governance to resolve appeal (approve)", async function () {
      const {
        slashingManager,
        slasher,
        operator,
        owner,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );
      await slashingManager.connect(operator).fileAppeal(0, "Evidence");

      const resolution = "Appeal approved after review";

      await expect(
        slashingManager.connect(owner).resolveAppeal(0, true, resolution),
      )
        .to.emit(slashingManager, "AppealResolved")
        .withArgs(
          0,
          operatorAddress,
          true,
          await owner.getAddress(),
          resolution,
        );

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.resolved).to.be.true;
      expect(proposal.approved).to.be.true;
    });

    it("should allow governance to resolve appeal (deny)", async function () {
      const {
        slashingManager,
        slasher,
        operator,
        owner,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );
      await slashingManager.connect(operator).fileAppeal(0, "Evidence");

      await slashingManager
        .connect(owner)
        .resolveAppeal(0, false, "Appeal denied");

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.resolved).to.be.true;
      expect(proposal.approved).to.be.false;
    });

    it("should block execution if appeal is pending", async function () {
      const {
        slashingManager,
        slasher,
        operator,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );
      await slashingManager.connect(operator).fileAppeal(0, "Evidence");

      await time.increase(7 * 24 * 60 * 60 + 1);

      await expect(
        slashingManager.connect(slasher).executeSlash(0),
      ).to.be.revertedWithCustomError(slashingManager, "AppealPending");
    });

    it("should block execution if appeal was approved", async function () {
      const {
        slashingManager,
        slasher,
        operator,
        owner,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );
      await slashingManager.connect(operator).fileAppeal(0, "Evidence");
      await slashingManager.connect(owner).resolveAppeal(0, true, "Approved");

      await time.increase(7 * 24 * 60 * 60 + 1);

      await expect(
        slashingManager.connect(slasher).executeSlash(0),
      ).to.be.revertedWithCustomError(slashingManager, "AppealUpheld");
    });

    it("should allow execution if appeal was denied", async function () {
      const {
        slashingManager,
        slasher,
        operator,
        owner,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlash(
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes(""),
        );
      await slashingManager.connect(operator).fileAppeal(0, "Evidence");
      await slashingManager.connect(owner).resolveAppeal(0, false, "Denied");

      await time.increase(7 * 24 * 60 * 60 + 1);

      await expect(slashingManager.connect(slasher).executeSlash(0)).to.emit(
        slashingManager,
        "SlashExecuted",
      );
    });
  });

  describe("ban management", function () {
    it("should allow governance to ban node", async function () {
      const { slashingManager, owner, operatorAddress } =
        await loadFixture(setup);

      const reason = ethers.encodeBytes32String("manual_ban");

      await expect(
        slashingManager.connect(owner).banNode(operatorAddress, reason),
      )
        .to.emit(slashingManager, "NodeBanned")
        .withArgs(operatorAddress, reason, await owner.getAddress());

      expect(await slashingManager.isBanned(operatorAddress)).to.be.true;
    });

    it("should allow governance to unban node", async function () {
      const { slashingManager, owner, operatorAddress } =
        await loadFixture(setup);

      await slashingManager
        .connect(owner)
        .banNode(operatorAddress, ethers.encodeBytes32String("test"));
      expect(await slashingManager.isBanned(operatorAddress)).to.be.true;

      await expect(slashingManager.connect(owner).unbanNode(operatorAddress))
        .to.emit(slashingManager, "NodeUnbanned")
        .withArgs(operatorAddress, await owner.getAddress());

      expect(await slashingManager.isBanned(operatorAddress)).to.be.false;
    });

    it("should revert if non-governance tries to ban", async function () {
      const { slashingManager, notTheOwner, operatorAddress } =
        await loadFixture(setup);

      await expect(
        slashingManager
          .connect(notTheOwner)
          .banNode(operatorAddress, ethers.encodeBytes32String("test")),
      ).to.be.revertedWithCustomError(slashingManager, "Unauthorized");
    });

    it("should prevent proposing slashes against banned nodes", async function () {
      const { slashingManager, owner, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(owner)
        .banNode(operatorAddress, ethers.encodeBytes32String("test"));

      await expect(
        slashingManager
          .connect(slasher)
          .proposeSlash(
            operatorAddress,
            REASON_MISBEHAVIOR,
            ethers.toUtf8Bytes("proof"),
          ),
      ).to.be.revertedWithCustomError(slashingManager, "CiphernodeBanned");
    });
  });

  describe("view functions", function () {
    it("should return correct slash policy", async function () {
      const { slashingManager, mockVerifier } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: await mockVerifier.getAddress(),
        banNode: true,
        appealWindow: 0,
        enabled: true,
      };

      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, policy);

      const retrieved =
        await slashingManager.getSlashPolicy(REASON_MISBEHAVIOR);
      expect(retrieved.ticketPenalty).to.equal(policy.ticketPenalty);
      expect(retrieved.licensePenalty).to.equal(policy.licensePenalty);
      expect(retrieved.requiresProof).to.equal(policy.requiresProof);
      expect(retrieved.proofVerifier).to.equal(policy.proofVerifier);
      expect(retrieved.banNode).to.equal(policy.banNode);
      expect(retrieved.appealWindow).to.equal(policy.appealWindow);
      expect(retrieved.enabled).to.equal(policy.enabled);
    });

    it("should return correct slash proposal", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      const proof = ethers.toUtf8Bytes("test proof");
      await slashingManager
        .connect(slasher)
        .proposeSlash(operatorAddress, REASON_MISBEHAVIOR, proof);

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.operator).to.equal(operatorAddress);
      expect(proposal.reason).to.equal(REASON_MISBEHAVIOR);
      expect(proposal.ticketAmount).to.equal(ethers.parseUnits("50", 6));
      expect(proposal.licenseAmount).to.equal(ethers.parseEther("100"));
      expect(proposal.proposer).to.equal(await slasher.getAddress());
      expect(proposal.proofHash).to.equal(ethers.keccak256(proof));
      expect(proposal.proofVerified).to.be.true;
    });

    it("should revert for invalid proposal ID", async function () {
      const { slashingManager } = await loadFixture(setup);

      await expect(
        slashingManager.getSlashProposal(999),
      ).to.be.revertedWithCustomError(slashingManager, "InvalidProposal");
    });
  });
});
