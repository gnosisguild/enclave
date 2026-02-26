// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import { network } from "hardhat";

import BondingRegistryModule from "../../ignition/modules/bondingRegistry";
import EnclaveTicketTokenModule from "../../ignition/modules/enclaveTicketToken";
import EnclaveTokenModule from "../../ignition/modules/enclaveToken";
import MockCiphernodeRegistryModule from "../../ignition/modules/mockCiphernodeRegistry";
import MockCircuitVerifierModule from "../../ignition/modules/mockSlashingVerifier";
import MockStableTokenModule from "../../ignition/modules/mockStableToken";
import SlashingManagerModule from "../../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  EnclaveTicketToken__factory as EnclaveTicketTokenFactory,
  EnclaveToken__factory as EnclaveTokenFactory,
  MockCiphernodeRegistry__factory as MockCiphernodeRegistryFactory,
  MockCircuitVerifier__factory as MockCircuitVerifierFactory,
  MockUSDC__factory as MockUSDCFactory,
  SlashingManager__factory as SlashingManagerFactory,
} from "../../types";
import type { MockCircuitVerifier } from "../../types";
import type { SlashingManager } from "../../types/contracts/slashing/SlashingManager";

const { ethers, networkHelpers, ignition } = await network.connect();
const { loadFixture, time } = networkHelpers;

describe("SlashingManager", function () {
  const REASON_MISBEHAVIOR = ethers.encodeBytes32String("misbehavior");
  const REASON_INACTIVITY = ethers.encodeBytes32String("inactivity");
  const REASON_DOUBLE_SIGN = ethers.encodeBytes32String("doubleSign");

  const SLASHER_ROLE = ethers.keccak256(ethers.toUtf8Bytes("SLASHER_ROLE"));
  const GOVERNANCE_ROLE = ethers.keccak256(
    ethers.toUtf8Bytes("GOVERNANCE_ROLE"),
  );
  const DEFAULT_ADMIN_ROLE = ethers.ZeroHash;

  const APPEAL_WINDOW = 7 * 24 * 60 * 60;

  // Placeholder address for contracts not under test
  const addressOne = "0x0000000000000000000000000000000000000001";

  const abiCoder = ethers.AbiCoder.defaultAbiCoder();

  // Must match the PROOF_PAYLOAD_TYPEHASH in SlashingManager.sol
  const PROOF_PAYLOAD_TYPEHASH = ethers.keccak256(
    ethers.toUtf8Bytes(
      "ProofPayload(uint256 chainId,uint256 e3Id,uint256 proofType,bytes zkProof,bytes publicSignals)",
    ),
  );

  /**
   * Helper to create a signed proof evidence bundle.
   * The operator signs the proof payload (matching Rust ProofPayload.digest()),
   * then the evidence is encoded in the format expected by proposeSlash().
   * Returns abi.encode(zkProof, publicInputs, signature, chainId, proofType, verifier)
   */
  async function signAndEncodeProof(
    signer: any,
    e3Id: number,
    reason: string,
    verifierAddress: string,
    zkProof: string = "0x1234",
    publicInputs: string[] = [ethers.ZeroHash],
    chainId: number = 31337, // Hardhat default chain ID
    proofType: number = 0, // T0PkBfv
  ): Promise<string> {
    // Operator signs: keccak256(abi.encode(PROOF_PAYLOAD_TYPEHASH, chainId, e3Id, proofType, keccak256(zkProof), keccak256(publicSignals)))
    const messageHash = ethers.keccak256(
      abiCoder.encode(
        ["bytes32", "uint256", "uint256", "uint256", "bytes32", "bytes32"],
        [
          PROOF_PAYLOAD_TYPEHASH,
          chainId,
          e3Id,
          proofType,
          ethers.keccak256(zkProof),
          ethers.keccak256(
            ethers.solidityPacked(["bytes32[]"], [publicInputs]),
          ),
        ],
      ),
    );
    const signature = await signer.signMessage(ethers.getBytes(messageHash));
    // Evidence format: abi.encode(zkProof, publicInputs, signature, chainId, proofType, verifier)
    return abiCoder.encode(
      ["bytes", "bytes32[]", "bytes", "uint256", "uint256", "address"],
      [zkProof, publicInputs, signature, chainId, proofType, verifierAddress],
    );
  }

  /**
   * Legacy helper for tests that check early failures (before abi.decode).
   * This encodes a minimal 6-tuple with dummy values for basic validation tests.
   */
  function encodeDummyProof(
    zkProof: string = "0x1234",
    publicInputs: string[] = [ethers.ZeroHash],
    verifierAddress: string = ethers.ZeroAddress,
  ): string {
    return abiCoder.encode(
      ["bytes", "bytes32[]", "bytes", "uint256", "uint256", "address"],
      [zkProof, publicInputs, "0x00", 31337, 0, verifierAddress],
    );
  }

  async function setupPolicies(
    slashingManager: SlashingManager,
    mockVerifier: MockCircuitVerifier,
  ) {
    const proofPolicy = {
      ticketPenalty: ethers.parseUnits("50", 6),
      licensePenalty: ethers.parseEther("100"),
      requiresProof: true,
      proofVerifier: await mockVerifier.getAddress(),
      banNode: false,
      appealWindow: 0,
      enabled: true,
      affectsCommittee: false,
      failureReason: 0,
    };

    const evidencePolicy = {
      ticketPenalty: ethers.parseUnits("20", 6),
      licensePenalty: ethers.parseEther("50"),
      requiresProof: false,
      proofVerifier: ethers.ZeroAddress,
      banNode: false,
      appealWindow: APPEAL_WINDOW,
      enabled: true,
      affectsCommittee: false,
      failureReason: 0,
    };

    const banPolicy = {
      ticketPenalty: ethers.parseUnits("100", 6),
      licensePenalty: ethers.parseEther("500"),
      requiresProof: true,
      proofVerifier: await mockVerifier.getAddress(),
      banNode: true,
      appealWindow: 0,
      enabled: true,
      affectsCommittee: false,
      failureReason: 0,
    };

    await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);
    await slashingManager.setSlashPolicy(REASON_INACTIVITY, evidencePolicy);
    await slashingManager.setSlashPolicy(REASON_DOUBLE_SIGN, banPolicy);
  }

  async function setup() {
    // ── Signers ────────────────────────────────────────────────────────────────
    const [owner, slasher, proposer, operator, notTheOwner] =
      await ethers.getSigners();
    const ownerAddress = await owner.getAddress();
    const operatorAddress = await operator.getAddress();

    // ── Token Contracts ────────────────────────────────────────────────────────
    const { mockUSDC } = await ignition.deploy(MockStableTokenModule, {
      parameters: { MockUSDC: { initialSupply: 1_000_000 } },
    });
    const { enclaveToken: _enclaveToken } = await ignition.deploy(
      EnclaveTokenModule,
      {
        parameters: { EnclaveToken: { owner: ownerAddress } },
      },
    );
    const { enclaveTicketToken } = await ignition.deploy(
      EnclaveTicketTokenModule,
      {
        parameters: {
          EnclaveTicketToken: {
            baseToken: await mockUSDC.getAddress(),
            registry: ownerAddress,
            owner: ownerAddress,
          },
        },
      },
    );

    // ── Mock Contracts ─────────────────────────────────────────────────────────
    const { mockCircuitVerifier } = await ignition.deploy(
      MockCircuitVerifierModule,
    );
    const { mockCiphernodeRegistry: _mockCiphernodeRegistry } =
      await ignition.deploy(MockCiphernodeRegistryModule);
    const mockCiphernodeRegistryAddress =
      await _mockCiphernodeRegistry.getAddress();

    // ── Slashing & Bonding ─────────────────────────────────────────────────────
    const { slashingManager: _slashingManager } = await ignition.deploy(
      SlashingManagerModule,
      {
        parameters: {
          SlashingManager: {
            admin: ownerAddress,
          },
        },
      },
    );

    const { bondingRegistry: _bondingRegistry } = await ignition.deploy(
      BondingRegistryModule,
      {
        parameters: {
          BondingRegistry: {
            owner: ownerAddress,
            ticketToken: await enclaveTicketToken.getAddress(),
            licenseToken: await _enclaveToken.getAddress(),
            registry: ethers.ZeroAddress,
            slashedFundsTreasury: ownerAddress,
            ticketPrice: ethers.parseUnits("10", 6),
            licenseRequiredBond: ethers.parseEther("1000"),
            minTicketBalance: 5,
            exitDelay: APPEAL_WINDOW,
          },
        },
      },
    );

    // ── Connect Factories ──────────────────────────────────────────────────────
    const usdcToken = MockUSDCFactory.connect(
      await mockUSDC.getAddress(),
      owner,
    );
    const enclaveToken = EnclaveTokenFactory.connect(
      await _enclaveToken.getAddress(),
      owner,
    );
    const ticketToken = EnclaveTicketTokenFactory.connect(
      await enclaveTicketToken.getAddress(),
      owner,
    );
    const mockVerifier = MockCircuitVerifierFactory.connect(
      await mockCircuitVerifier.getAddress(),
      owner,
    );
    const mockCiphernodeRegistry = MockCiphernodeRegistryFactory.connect(
      mockCiphernodeRegistryAddress,
      owner,
    );
    const slashingManager = SlashingManagerFactory.connect(
      await _slashingManager.getAddress(),
      owner,
    );
    const bondingRegistry = BondingRegistryFactory.connect(
      await _bondingRegistry.getAddress(),
      owner,
    );

    // ── Wire Up & Configure ────────────────────────────────────────────────────
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
    await slashingManager.setCiphernodeRegistry(mockCiphernodeRegistryAddress);
    await slashingManager.setEnclave(addressOne);
    await slashingManager.setE3RefundManager(addressOne);

    // ── Return ─────────────────────────────────────────────────────────────────
    return {
      owner,
      slasher,
      proposer,
      operator,
      operatorAddress,
      notTheOwner,
      slashingManager,
      bondingRegistry,
      enclaveToken,
      ticketToken,
      usdcToken,
      mockVerifier,
      mockCiphernodeRegistry,
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
              ciphernodeRegistry: ethers.ZeroAddress,
              enclave: ethers.ZeroAddress,
            },
          },
        }),
      ).to.be.rejected;
    });
  });

  describe("setSlashPolicy()", function () {
    it("should set a valid proof-based slash policy", async function () {
      const { slashingManager, mockVerifier } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: await mockVerifier.getAddress(),
        banNode: false,
        appealWindow: 0,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
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

    it("should set an evidence-based policy (no proof required)", async function () {
      const { slashingManager } = await loadFixture(setup);

      const policy = {
        ticketPenalty: ethers.parseUnits("20", 6),
        licensePenalty: ethers.parseEther("50"),
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: APPEAL_WINDOW,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
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
        appealWindow: APPEAL_WINDOW,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
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
        appealWindow: APPEAL_WINDOW,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
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
        appealWindow: APPEAL_WINDOW,
        enabled: false,
        affectsCommittee: false,
        failureReason: 0,
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
        appealWindow: APPEAL_WINDOW,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
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
        affectsCommittee: false,
        failureReason: 0,
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
        appealWindow: APPEAL_WINDOW,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
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
        affectsCommittee: false,
        failureReason: 0,
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

    it("should revert if non-admin tries to add slasher", async function () {
      const { slashingManager, notTheOwner } = await loadFixture(setup);

      await expect(
        slashingManager
          .connect(notTheOwner)
          .addSlasher(await notTheOwner.getAddress()),
      ).to.be.revertedWithCustomError(
        slashingManager,
        "AccessControlUnauthorizedAccount",
      );
    });

    it("should revert if zero address is added as slasher", async function () {
      const { slashingManager } = await loadFixture(setup);

      await expect(
        slashingManager.addSlasher(ethers.ZeroAddress),
      ).to.be.revertedWithCustomError(slashingManager, "ZeroAddress");
    });
  });

  describe("proposeSlash() — Lane A (proof-based, permissionless)", function () {
    it("should propose and auto-execute slash with signed proof from operator", async function () {
      const {
        slashingManager,
        proposer,
        operator,
        operatorAddress,
        mockVerifier,
        mockCiphernodeRegistry,
      } = await loadFixture(setup);

      // MockCircuitVerifier default returnValue=false → proof invalid → fault confirmed
      const verifierAddress = await mockVerifier.getAddress();
      const proofPolicy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: verifierAddress,
        banNode: false,
        appealWindow: 0,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
      };
      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);

      // Set up committee membership for operator
      const e3Id = 0;
      await mockCiphernodeRegistry.setCommitteeNodes(e3Id, [operatorAddress]);

      // Operator signs the bad proof
      const proof = await signAndEncodeProof(
        operator,
        e3Id,
        REASON_MISBEHAVIOR,
        verifierAddress,
      );

      // Anyone can submit the signed evidence (permissionless for Lane A)
      await expect(
        slashingManager
          .connect(proposer)
          .proposeSlash(e3Id, operatorAddress, REASON_MISBEHAVIOR, proof),
      ).to.emit(slashingManager, "SlashProposed");

      // Proof-based slashes auto-execute
      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.operator).to.equal(operatorAddress);
      expect(proposal.reason).to.equal(REASON_MISBEHAVIOR);
      expect(proposal.proofVerified).to.be.true;
      expect(proposal.executed).to.be.true;
      expect(proposal.proposer).to.equal(await proposer.getAddress());
    });

    it("should revert if circuit verifier says proof is valid (no fault)", async function () {
      const {
        slashingManager,
        proposer,
        operator,
        operatorAddress,
        mockVerifier,
        mockCiphernodeRegistry,
      } = await loadFixture(setup);

      const verifierAddress = await mockVerifier.getAddress();
      const proofPolicy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: verifierAddress,
        banNode: false,
        appealWindow: 0,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
      };
      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);

      // Set mock verifier to return true → proof is valid → NOT a fault
      await mockVerifier.setReturnValue(true);

      await mockCiphernodeRegistry.setCommitteeNodes(0, [operatorAddress]);

      const proof = await signAndEncodeProof(
        operator,
        0,
        REASON_MISBEHAVIOR,
        verifierAddress,
      );
      await expect(
        slashingManager
          .connect(proposer)
          .proposeSlash(0, operatorAddress, REASON_MISBEHAVIOR, proof),
      ).to.be.revertedWithCustomError(slashingManager, "ProofIsValid");
    });

    it("should revert if signer is not the operator (V-001 fix)", async function () {
      const {
        slashingManager,
        proposer,
        operatorAddress,
        mockVerifier,
        mockCiphernodeRegistry,
      } = await loadFixture(setup);

      const verifierAddress = await mockVerifier.getAddress();
      const proofPolicy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: verifierAddress,
        banNode: false,
        appealWindow: 0,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
      };
      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);

      await mockCiphernodeRegistry.setCommitteeNodes(0, [operatorAddress]);

      // Proposer signs the proof (NOT the operator) — should be rejected
      const proof = await signAndEncodeProof(
        proposer,
        0,
        REASON_MISBEHAVIOR,
        verifierAddress,
      );
      await expect(
        slashingManager
          .connect(proposer)
          .proposeSlash(0, operatorAddress, REASON_MISBEHAVIOR, proof),
      ).to.be.revertedWithCustomError(slashingManager, "SignerIsNotOperator");
    });

    it("should revert if operator is not in committee (V-001 fix)", async function () {
      const {
        slashingManager,
        proposer,
        operator,
        operatorAddress,
        mockVerifier,
      } = await loadFixture(setup);

      const verifierAddress = await mockVerifier.getAddress();
      const proofPolicy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: verifierAddress,
        banNode: false,
        appealWindow: 0,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
      };
      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);

      // Do NOT add operator to committee — empty committee for this E3

      const proof = await signAndEncodeProof(
        operator,
        0,
        REASON_MISBEHAVIOR,
        verifierAddress,
      );
      await expect(
        slashingManager
          .connect(proposer)
          .proposeSlash(0, operatorAddress, REASON_MISBEHAVIOR, proof),
      ).to.be.revertedWithCustomError(
        slashingManager,
        "OperatorNotInCommittee",
      );
    });

    it("should revert if operator is zero address", async function () {
      const { slashingManager, proposer, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      // Any non-empty proof triggers ZeroAddress check before decode
      const proof = encodeDummyProof();

      await expect(
        slashingManager
          .connect(proposer)
          .proposeSlash(0, ethers.ZeroAddress, REASON_MISBEHAVIOR, proof),
      ).to.be.revertedWithCustomError(slashingManager, "ZeroAddress");
    });

    it("should revert if slash reason is disabled", async function () {
      const { slashingManager, proposer, operatorAddress } =
        await loadFixture(setup);

      const proof = encodeDummyProof();

      await expect(
        slashingManager
          .connect(proposer)
          .proposeSlash(0, operatorAddress, REASON_DOUBLE_SIGN, proof),
      ).to.be.revertedWithCustomError(slashingManager, "SlashReasonDisabled");
    });

    it("should revert if proof is empty", async function () {
      const { slashingManager, proposer, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      const proofPolicy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: await mockVerifier.getAddress(),
        banNode: false,
        appealWindow: 0,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
      };
      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);

      await expect(
        slashingManager
          .connect(proposer)
          .proposeSlash(0, operatorAddress, REASON_MISBEHAVIOR, "0x"),
      ).to.be.revertedWithCustomError(slashingManager, "ProofRequired");
    });

    it("should reject duplicate evidence", async function () {
      const {
        slashingManager,
        proposer,
        operator,
        operatorAddress,
        mockVerifier,
        mockCiphernodeRegistry,
      } = await loadFixture(setup);

      const verifierAddress = await mockVerifier.getAddress();
      const proofPolicy = {
        ticketPenalty: ethers.parseUnits("50", 6),
        licensePenalty: ethers.parseEther("100"),
        requiresProof: true,
        proofVerifier: verifierAddress,
        banNode: false,
        appealWindow: 0,
        enabled: true,
        affectsCommittee: false,
        failureReason: 0,
      };
      await slashingManager.setSlashPolicy(REASON_MISBEHAVIOR, proofPolicy);
      await mockCiphernodeRegistry.setCommitteeNodes(0, [operatorAddress]);

      const proof = await signAndEncodeProof(
        operator,
        0,
        REASON_MISBEHAVIOR,
        verifierAddress,
      );
      await slashingManager
        .connect(proposer)
        .proposeSlash(0, operatorAddress, REASON_MISBEHAVIOR, proof);

      // Same proof for same e3Id/operator/reason should be rejected
      await expect(
        slashingManager
          .connect(proposer)
          .proposeSlash(0, operatorAddress, REASON_MISBEHAVIOR, proof),
      ).to.be.revertedWithCustomError(slashingManager, "DuplicateEvidence");
    });

    it("should increment totalProposals", async function () {
      const {
        slashingManager,
        proposer,
        operator,
        operatorAddress,
        mockVerifier,
        mockCiphernodeRegistry,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      const verifierAddress = await mockVerifier.getAddress();
      await mockCiphernodeRegistry.setCommitteeNodes(0, [operatorAddress]);
      await mockCiphernodeRegistry.setCommitteeNodes(1, [operatorAddress]);

      expect(await slashingManager.totalProposals()).to.equal(0);

      const proof1 = await signAndEncodeProof(
        operator,
        0,
        REASON_MISBEHAVIOR,
        verifierAddress,
        "0x1111",
      );
      await slashingManager
        .connect(proposer)
        .proposeSlash(0, operatorAddress, REASON_MISBEHAVIOR, proof1);

      expect(await slashingManager.totalProposals()).to.equal(1);

      const proof2 = await signAndEncodeProof(
        operator,
        1,
        REASON_MISBEHAVIOR,
        verifierAddress,
        "0x2222",
      );
      await slashingManager
        .connect(proposer)
        .proposeSlash(1, operatorAddress, REASON_MISBEHAVIOR, proof2);

      expect(await slashingManager.totalProposals()).to.equal(2);
    });

    it("should ban node when policy requires it", async function () {
      const {
        slashingManager,
        proposer,
        operator,
        operatorAddress,
        mockVerifier,
        mockCiphernodeRegistry,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      const verifierAddress = await mockVerifier.getAddress();
      await mockCiphernodeRegistry.setCommitteeNodes(0, [operatorAddress]);

      expect(await slashingManager.isBanned(operatorAddress)).to.be.false;

      const proof = await signAndEncodeProof(
        operator,
        0,
        REASON_DOUBLE_SIGN,
        verifierAddress,
        "0x3333",
      );
      await slashingManager
        .connect(proposer)
        .proposeSlash(0, operatorAddress, REASON_DOUBLE_SIGN, proof);

      // banNode=true → auto-executed → node is now banned
      expect(await slashingManager.isBanned(operatorAddress)).to.be.true;
    });
  });

  describe("proposeSlashEvidence() — Lane B (evidence-based, SLASHER_ROLE)", function () {
    it("should propose evidence-based slash with appeal window", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      const evidence = ethers.toUtf8Bytes("operator was inactive during E3");
      const e3Id = 0;

      await expect(
        slashingManager
          .connect(slasher)
          .proposeSlashEvidence(
            e3Id,
            operatorAddress,
            REASON_INACTIVITY,
            evidence,
          ),
      ).to.emit(slashingManager, "SlashProposed");

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.operator).to.equal(operatorAddress);
      expect(proposal.reason).to.equal(REASON_INACTIVITY);
      expect(proposal.proofVerified).to.be.false;
      expect(proposal.executed).to.be.false;
      expect(proposal.proposer).to.equal(await slasher.getAddress());
      expect(proposal.executableAt).to.be.gt(proposal.proposedAt);
    });

    it("should revert if caller is not slasher", async function () {
      const { slashingManager, notTheOwner, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      const evidence = ethers.toUtf8Bytes("evidence");

      await expect(
        slashingManager
          .connect(notTheOwner)
          .proposeSlashEvidence(
            0,
            operatorAddress,
            REASON_INACTIVITY,
            evidence,
          ),
      ).to.be.revertedWithCustomError(slashingManager, "Unauthorized");
    });

    it("should revert if operator is zero address", async function () {
      const { slashingManager, slasher, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await expect(
        slashingManager
          .connect(slasher)
          .proposeSlashEvidence(
            0,
            ethers.ZeroAddress,
            REASON_INACTIVITY,
            ethers.toUtf8Bytes(""),
          ),
      ).to.be.revertedWithCustomError(slashingManager, "ZeroAddress");
    });
  });

  describe("executeSlash() — Lane B execution", function () {
    it("should execute evidence-based slash after appeal window", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
        );

      // Should revert before appeal window expires
      await expect(
        slashingManager.executeSlash(0),
      ).to.be.revertedWithCustomError(slashingManager, "AppealWindowActive");

      // Fast forward past appeal window
      await time.increase(APPEAL_WINDOW + 1);

      await expect(slashingManager.executeSlash(0)).to.emit(
        slashingManager,
        "SlashExecuted",
      );

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.executed).to.be.true;
    });

    it("should revert if proof-based slash tries to executeSlash separately", async function () {
      const {
        slashingManager,
        proposer,
        operator,
        operatorAddress,
        mockVerifier,
        mockCiphernodeRegistry,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);
      const verifierAddress = await mockVerifier.getAddress();
      await mockCiphernodeRegistry.setCommitteeNodes(0, [operatorAddress]);

      // Proof-based slash auto-executes in proposeSlash
      const proof = await signAndEncodeProof(
        operator,
        0,
        REASON_MISBEHAVIOR,
        verifierAddress,
      );
      await slashingManager
        .connect(proposer)
        .proposeSlash(0, operatorAddress, REASON_MISBEHAVIOR, proof);

      // Should revert because already executed
      await expect(
        slashingManager.executeSlash(0),
      ).to.be.revertedWithCustomError(slashingManager, "AlreadyExecuted");
    });

    it("should revert if proposal doesn't exist", async function () {
      const { slashingManager } = await loadFixture(setup);

      await expect(
        slashingManager.executeSlash(999),
      ).to.be.revertedWithCustomError(slashingManager, "InvalidProposal");
    });

    it("should revert if already executed", async function () {
      const { slashingManager, slasher, operatorAddress, mockVerifier } =
        await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);

      await slashingManager
        .connect(slasher)
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
        );

      await time.increase(APPEAL_WINDOW + 1);
      await slashingManager.executeSlash(0);

      await expect(
        slashingManager.executeSlash(0),
      ).to.be.revertedWithCustomError(slashingManager, "AlreadyExecuted");
    });
  });

  describe("appeal system", function () {
    it("should allow operator to file appeal on evidence-based slash", async function () {
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
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
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
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
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
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
        );

      await time.increase(APPEAL_WINDOW + 1);

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
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
        );

      await slashingManager.connect(operator).fileAppeal(0, "First appeal");

      await expect(
        slashingManager.connect(operator).fileAppeal(0, "Second appeal"),
      ).to.be.revertedWithCustomError(slashingManager, "AlreadyAppealed");
    });

    it("should revert if appealing proof-verified slash", async function () {
      const {
        slashingManager,
        proposer,
        operator,
        operatorAddress,
        mockVerifier,
        mockCiphernodeRegistry,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);
      const verifierAddress = await mockVerifier.getAddress();
      await mockCiphernodeRegistry.setCommitteeNodes(0, [operatorAddress]);

      // Proof-based slash auto-executes with proofVerified=true
      const proof = await signAndEncodeProof(
        operator,
        0,
        REASON_MISBEHAVIOR,
        verifierAddress,
      );
      await slashingManager
        .connect(proposer)
        .proposeSlash(0, operatorAddress, REASON_MISBEHAVIOR, proof);

      // Cannot appeal proof-verified slashes — appeal window is 0 so it's already expired
      await expect(
        slashingManager.connect(operator).fileAppeal(0, "Cannot appeal proof"),
      ).to.be.revertedWithCustomError(slashingManager, "AppealWindowExpired");
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
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
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
      expect(proposal.appealUpheld).to.be.true;
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
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
        );
      await slashingManager.connect(operator).fileAppeal(0, "Evidence");

      await slashingManager
        .connect(owner)
        .resolveAppeal(0, false, "Appeal denied");

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.resolved).to.be.true;
      expect(proposal.appealUpheld).to.be.false;
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
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
        );
      await slashingManager.connect(operator).fileAppeal(0, "Evidence");

      await time.increase(APPEAL_WINDOW + 1);

      await expect(
        slashingManager.executeSlash(0),
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
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
        );
      await slashingManager.connect(operator).fileAppeal(0, "Evidence");
      await slashingManager.connect(owner).resolveAppeal(0, true, "Approved");

      await time.increase(APPEAL_WINDOW + 1);

      await expect(
        slashingManager.executeSlash(0),
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
        .proposeSlashEvidence(
          0,
          operatorAddress,
          REASON_INACTIVITY,
          ethers.toUtf8Bytes("evidence"),
        );
      await slashingManager.connect(operator).fileAppeal(0, "Evidence");
      await slashingManager.connect(owner).resolveAppeal(0, false, "Denied");

      await time.increase(APPEAL_WINDOW + 1);

      await expect(slashingManager.executeSlash(0)).to.emit(
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
        slashingManager
          .connect(owner)
          .updateBanStatus(operatorAddress, true, reason),
      )
        .to.emit(slashingManager, "NodeBanUpdated")
        .withArgs(operatorAddress, true, reason, await owner.getAddress());

      expect(await slashingManager.isBanned(operatorAddress)).to.be.true;
    });

    it("should allow governance to unban node", async function () {
      const { slashingManager, owner, operatorAddress } =
        await loadFixture(setup);

      await slashingManager
        .connect(owner)
        .updateBanStatus(
          operatorAddress,
          true,
          ethers.encodeBytes32String("test"),
        );
      expect(await slashingManager.isBanned(operatorAddress)).to.be.true;

      await expect(
        slashingManager
          .connect(owner)
          .updateBanStatus(
            operatorAddress,
            false,
            ethers.encodeBytes32String("test"),
          ),
      )
        .to.emit(slashingManager, "NodeBanUpdated")
        .withArgs(
          operatorAddress,
          false,
          ethers.encodeBytes32String("test"),
          await owner.getAddress(),
        );

      expect(await slashingManager.isBanned(operatorAddress)).to.be.false;
    });

    it("should revert if non-governance tries to ban", async function () {
      const { slashingManager, notTheOwner, operatorAddress } =
        await loadFixture(setup);

      await expect(
        slashingManager
          .connect(notTheOwner)
          .updateBanStatus(
            operatorAddress,
            false,
            ethers.encodeBytes32String("test"),
          ),
      ).to.be.revertedWithCustomError(slashingManager, "Unauthorized");
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
        affectsCommittee: false,
        failureReason: 0,
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
      expect(retrieved.affectsCommittee).to.equal(policy.affectsCommittee);
      expect(retrieved.failureReason).to.equal(policy.failureReason);
    });

    it("should return correct slash proposal", async function () {
      const {
        slashingManager,
        proposer,
        operator,
        operatorAddress,
        mockVerifier,
        mockCiphernodeRegistry,
      } = await loadFixture(setup);

      await setupPolicies(slashingManager, mockVerifier);
      const verifierAddress = await mockVerifier.getAddress();
      await mockCiphernodeRegistry.setCommitteeNodes(0, [operatorAddress]);

      const proof = await signAndEncodeProof(
        operator,
        0,
        REASON_MISBEHAVIOR,
        verifierAddress,
        "0x4444",
      );
      await slashingManager
        .connect(proposer)
        .proposeSlash(0, operatorAddress, REASON_MISBEHAVIOR, proof);

      const proposal = await slashingManager.getSlashProposal(0);
      expect(proposal.operator).to.equal(operatorAddress);
      expect(proposal.reason).to.equal(REASON_MISBEHAVIOR);
      expect(proposal.ticketAmount).to.equal(ethers.parseUnits("50", 6));
      expect(proposal.licenseAmount).to.equal(ethers.parseEther("100"));
      expect(proposal.proposer).to.equal(await proposer.getAddress());
      expect(proposal.proofHash).to.equal(ethers.keccak256(proof));
      expect(proposal.proofVerified).to.be.true;
      expect(proposal.executed).to.be.true;
    });

    it("should revert for invalid proposal ID", async function () {
      const { slashingManager } = await loadFixture(setup);

      await expect(
        slashingManager.getSlashProposal(999),
      ).to.be.revertedWithCustomError(slashingManager, "InvalidProposal");
    });
  });
});
