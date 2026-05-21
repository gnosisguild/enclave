// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/**
 * Tests for committee expulsion, viability checks, and E3 failure on threshold breach.
 *
 * Verifies:
 * - Committee members are expelled via proposeSlash when affectsCommittee=true
 * - The E3 continues as long as active members >= threshold M
 * - The E3 fails when active members drop below threshold M
 * - Rewards exclude expelled members
 * - Idempotent expulsion (re-slashing same node doesn't double-count)
 */
import { expect } from "chai";
import type { Signer } from "ethers";

import {
  LARGE_TIMEOUT_CONFIG,
  ONE_DAY,
  SORTITION_SUBMISSION_WINDOW,
  deployEnclaveSystem,
  ethers,
  networkHelpers,
  signAndEncodeAttestation,
} from "../fixtures";

const { loadFixture, time } = networkHelpers;

describe("Committee Expulsion & Fault Tolerance", function () {
  // Lane A reasons are derived on-chain as keccak256(abi.encodePacked(proofType))
  const REASON_PT_0 = ethers.keccak256(ethers.solidityPacked(["uint256"], [0]));
  const REASON_PT_7 = ethers.keccak256(ethers.solidityPacked(["uint256"], [7]));

  const abiCoder = ethers.AbiCoder.defaultAbiCoder();

  const setup = async () => {
    const signers = await ethers.getSigners();
    const [
      owner,
      requester,
      treasury,
      operator1,
      operator2,
      operator3,
      operator4,
    ] = signers;
    const requesterAddress = await requester.getAddress();

    const sys = await deployEnclaveSystem({
      bfvParams: "large",
      timeoutConfig: LARGE_TIMEOUT_CONFIG,
      committeeThresholds: [
        [0, [1, 3]], // Micro
        [1, [2, 3]], // Small
        [2, [2, 4]], // Medium
      ],
      deployCircuitVerifier: true,
      setupOperators: 0,
      slashedFundsTreasury: treasury,
      mintUsdcTo: [],
    });
    const {
      enclave,
      ciphernodeRegistry: registry,
      slashingManager,
      bondingRegistry,
      licenseToken: enclToken,
      ticketToken,
      usdcToken,
      mocks,
    } = sys;
    const mockVerifier = mocks.circuitVerifier!;
    const e3Program = mocks.e3Program;
    const decryptionVerifier = mocks.decryptionVerifier;
    const enclaveAddress = await enclave.getAddress();

    // Fund the requester (fixture's `mintUsdcTo: []` skipped this).
    await usdcToken.mint(requesterAddress, ethers.parseUnits("100000", 6));

    // ── Slash Policies ─────────────────────────────────────────────────────
    const baseSlashPolicy = {
      ticketPenalty: ethers.parseUnits("10", 6),
      licensePenalty: ethers.parseEther("50"),
      requiresProof: true,
      proofVerifier: ethers.ZeroAddress,
      banNode: false,
      appealWindow: 0,
      enabled: true,
      affectsCommittee: true,
    };

    await slashingManager.setSlashPolicy(REASON_PT_0, {
      ...baseSlashPolicy,
      failureReason: 4, // FailureReason.DKGInvalidShares
    });
    await slashingManager.setSlashPolicy(REASON_PT_7, {
      ...baseSlashPolicy,
      failureReason: 11, // FailureReason.DecryptionInvalidShares
    });

    // ── Helpers ────────────────────────────────────────────────────────────
    async function setupOperator(operator: Signer) {
      const operatorAddress = await operator.getAddress();

      await enclToken.mintAllocation(
        operatorAddress,
        ethers.parseEther("10000"),
        "Test allocation",
      );
      await usdcToken.mint(operatorAddress, ethers.parseUnits("100000", 6));

      await enclToken
        .connect(operator)
        .approve(await bondingRegistry.getAddress(), ethers.parseEther("2000"));
      await bondingRegistry
        .connect(operator)
        .bondLicense(ethers.parseEther("1000"));
      await bondingRegistry.connect(operator).registerOperator();

      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator)
        .approve(await bondingRegistry.ticketToken(), ticketAmount);
      await bondingRegistry.connect(operator).addTicketBalance(ticketAmount);
    }

    async function makeRequest(committeeSize: number = 1) {
      const startTime = (await time.latest()) + 100;
      const requestParams = {
        committeeSize,
        inputWindow: [startTime + 100, startTime + ONE_DAY] as [number, number],
        e3Program: await e3Program.getAddress(),
        paramSet: 0,
        computeProviderParams: abiCoder.encode(
          ["address"],
          [await decryptionVerifier.getAddress()],
        ),
        customParams: abiCoder.encode(
          ["address"],
          ["0x1234567890123456789012345678901234567890"],
        ),
        proofAggregationEnabled: false,
      };

      const fee = await enclave.getE3Quote(requestParams);
      await usdcToken.connect(requester).approve(enclaveAddress, fee);
      await enclave.connect(requester).request(requestParams);
    }

    async function finalizeCommitteeWithOperators(
      e3Id: number,
      operators: Signer[],
    ) {
      for (const op of operators)
        await registry.connect(op).submitTicket(e3Id, 1);

      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(e3Id);

      const nodes = await Promise.all(operators.map((op) => op.getAddress()));
      const publicKey = ethers.toUtf8Bytes("fake-public-key");
      const pkCommitment = ethers.keccak256(publicKey);
      await registry.publishCommittee(
        e3Id,
        nodes,
        publicKey,
        pkCommitment,
        "0x",
      );
    }

    return {
      enclave,
      registry,
      slashingManager,
      bondingRegistry,
      mockVerifier,
      usdcToken,
      enclToken,
      ticketToken,
      owner,
      requester,
      treasury,
      operator1,
      operator2,
      operator3,
      operator4,
      setupOperator,
      makeRequest,
      finalizeCommitteeWithOperators,
    };
  };

  describe("committee expulsion via proposeSlash", function () {
    it("should expel a committee member and emit CommitteeMemberExpelled", async function () {
      const {
        registry,
        slashingManager,
        operator1,
        operator2,
        operator3,
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);
      await setupOperator(operator3);

      // Small (committeeSize=1) means M=2, N=3
      await makeRequest(1);
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
      ]);

      const op1Address = await operator1.getAddress();

      // Verify member is active before slash
      expect(await registry.isCommitteeMemberActive(0, op1Address)).to.be.true;
      expect((await registry.getCommitteeViability(0)).activeCount).to.equal(3);

      // Committee members attest that operator1 is faulty
      const proof = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        op1Address,
        await slashingManager.getAddress(),
      );
      const tx = await slashingManager.proposeSlash(0, op1Address, proof);

      // Should emit CommitteeMemberExpelled
      await expect(tx)
        .to.emit(registry, "CommitteeMemberExpelled")
        .withArgs(0, op1Address, REASON_PT_0, 2);

      // Should emit CommitteeViabilityUpdated
      await expect(tx)
        .to.emit(registry, "CommitteeViabilityUpdated")
        .withArgs(0, 2, 2, true); // 2 >= 2 → viable

      // Verify member is no longer active
      expect(await registry.isCommitteeMemberActive(0, op1Address)).to.be.false;
      expect((await registry.getCommitteeViability(0)).activeCount).to.equal(2);
    });

    it("should keep E3 alive when active members >= threshold", async function () {
      const {
        enclave,
        registry,
        slashingManager,
        operator1,
        operator2,
        operator3,
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);
      await setupOperator(operator3);

      await makeRequest(1); // Small: M=2, N=3
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
      ]);

      // Slash one member — 3 active → 2 active, threshold is 2, still viable
      const proof = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
        await slashingManager.getAddress(),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        proof,
      );

      // E3 should NOT be failed — stage should still be Requested (1)
      // or whatever stage it was at, not Failed
      const stage = await enclave.getE3Stage(0);
      expect(stage).to.not.equal(6); // 6 = E3Stage.Failed

      // Active committee still has enough members
      const { activeCount, thresholdM } =
        await registry.getCommitteeViability(0);
      expect(activeCount).to.equal(2);
      expect(thresholdM).to.equal(2); // M=2
    });

    it("should fail E3 when active members drop below threshold", async function () {
      const {
        enclave,
        slashingManager,
        owner,
        operator1,
        operator2,
        operator3,
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);
      await setupOperator(operator3);

      // Add an evidence-based slash policy (Lane B) with no appeal window
      const REASON_EVIDENCE = ethers.keccak256(
        ethers.toUtf8Bytes("E3_EVIDENCE_SLASH"),
      );
      await slashingManager.setSlashPolicy(REASON_EVIDENCE, {
        ticketPenalty: ethers.parseUnits("10", 6),
        licensePenalty: ethers.parseEther("50"),
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 1, // Minimum appeal window (1 second)
        enabled: true,
        affectsCommittee: true,
        failureReason: 4, // FailureReason.DKGInvalidShares
      });

      // Grant SLASHER_ROLE to owner for Lane B
      const SLASHER_ROLE = await slashingManager.SLASHER_ROLE();
      await slashingManager.grantRole(SLASHER_ROLE, await owner.getAddress());

      await makeRequest(1); // Small: M=2, N=3
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
      ]);

      // Lane A: Slash op1 with attestation from [op2, op3] — active 3→2, still >= M=2
      const proof = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
        await slashingManager.getAddress(),
        0,
        31337,
        ethers.keccak256(ethers.toUtf8Bytes("data1")),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        proof,
      );

      let stage = await enclave.getE3Stage(0);
      expect(stage).to.not.equal(6); // Not failed yet

      // Lane B: Evidence-based slash of op2 (no attestation needed) — active 2→1 < M=2
      // Lane A can't trigger E3 failure alone because you always need M active
      // non-accused voters, but after the slash active must drop below M — a contradiction.
      // Lane B (SLASHER_ROLE) bypasses attestation requirements for this final slash.
      const nextProposalId = await slashingManager.totalProposals();
      await slashingManager.proposeSlashEvidence(
        0,
        await operator2.getAddress(),
        REASON_EVIDENCE,
        ethers.toUtf8Bytes("evidence-data"),
      );

      // Wait for appeal window to pass, then execute
      await time.increase(2);
      const tx = await slashingManager.executeSlash(nextProposalId);

      // Should emit E3Failed event
      await expect(tx).to.emit(enclave, "E3Failed");

      // E3 should now be Failed
      stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(6); // E3Stage.Failed

      // Failure reason should be DKGInvalidShares (4)
      const reason = await enclave.getFailureReason(0);
      expect(reason).to.equal(4);
    });

    it("should handle idempotent expulsion (re-slashing same node)", async function () {
      const {
        registry,
        slashingManager,
        operator1,
        operator2,
        operator3,
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);
      await setupOperator(operator3);

      await makeRequest(1);
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
      ]);

      // Slash operator1 once
      const proof1 = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
        await slashingManager.getAddress(),
        0,
        31337,
        ethers.keccak256(ethers.toUtf8Bytes("first")),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        proof1,
      );
      expect((await registry.getCommitteeViability(0)).activeCount).to.equal(2);

      // Slash operator1 again for a different proof type to verify expulsion is idempotent.
      // Same (e3Id, operator, proofType) would revert DuplicateEvidence — that's correct.
      // Using proofType=7 (C6ThresholdShareDecryption) with REASON_PT_7 instead.
      const proof2 = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
        await slashingManager.getAddress(),
        7, // C6ThresholdShareDecryption — different proofType
        31337,
        ethers.keccak256(ethers.toUtf8Bytes("second")),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        proof2,
      );

      // Active count should still be 2 (idempotent expulsion)
      expect((await registry.getCommitteeViability(0)).activeCount).to.equal(2);
    });

    it("should exclude expelled members from getActiveCommitteeNodes", async function () {
      const {
        registry,
        slashingManager,
        operator1,
        operator2,
        operator3,
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);
      await setupOperator(operator3);

      await makeRequest(1);
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
      ]);

      // Before expulsion: all 3 should be in active nodes
      const [nodesBefore, scoresBefore] =
        await registry.getActiveCommitteeNodes(0);
      expect(nodesBefore.length).to.equal(3);
      expect(scoresBefore.length).to.equal(3);
      expect(nodesBefore).to.include(await operator1.getAddress());

      const scoreByNode = new Map(
        nodesBefore.map((node, index) => [
          node.toLowerCase(),
          scoresBefore[index],
        ]),
      );

      // Expel operator1
      const proof = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
        await slashingManager.getAddress(),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        proof,
      );

      // After expulsion: only 2 should be active
      const [nodesAfter, scoresAfter] =
        await registry.getActiveCommitteeNodes(0);
      expect(nodesAfter.length).to.equal(2);
      expect(scoresAfter.length).to.equal(2);
      expect(nodesAfter).to.not.include(await operator1.getAddress());
      expect(nodesAfter).to.include(await operator2.getAddress());
      expect(nodesAfter).to.include(await operator3.getAddress());
      scoresAfter.forEach((score, index) => {
        expect(score).to.equal(
          scoreByNode.get(nodesAfter[index].toLowerCase()),
        );
      });
    });
  });

  describe("E3 continues above threshold", function () {
    it("should allow multiple expulsions while staying above threshold", async function () {
      const {
        enclave,
        registry,
        slashingManager,
        operator1,
        operator2,
        operator3,
        operator4,
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);
      await setupOperator(operator3);
      await setupOperator(operator4);

      await makeRequest(2); // Medium: M=2, N=4
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
        operator4,
      ]);

      expect((await registry.getCommitteeViability(0)).activeCount).to.equal(4);

      // Expel 2 out of 4 — still have 2 >= M=2
      const proof1 = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
        await slashingManager.getAddress(),
        0,
        31337,
        ethers.keccak256(ethers.toUtf8Bytes("expel1")),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        proof1,
      );
      expect((await registry.getCommitteeViability(0)).activeCount).to.equal(3);

      const proof2 = await signAndEncodeAttestation(
        [operator3, operator4],
        0,
        await operator2.getAddress(),
        await slashingManager.getAddress(),
        0,
        31337,
        ethers.keccak256(ethers.toUtf8Bytes("expel2")),
      );
      await slashingManager.proposeSlash(
        0,
        await operator2.getAddress(),
        proof2,
      );
      expect((await registry.getCommitteeViability(0)).activeCount).to.equal(2);

      // E3 should NOT be failed
      const stage = await enclave.getE3Stage(0);
      expect(stage).to.not.equal(6);
    });
  });

  describe("E3 fails below threshold", function () {
    it("should fail E3 exactly at the threshold breach via Lane B", async function () {
      const {
        enclave,
        registry,
        slashingManager,
        owner,
        operator1,
        operator2,
        operator3,
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);
      await setupOperator(operator3);

      // Lane B evidence-based policy with no appeal window
      const REASON_EVIDENCE = ethers.keccak256(
        ethers.toUtf8Bytes("E3_EVIDENCE_SLASH"),
      );
      await slashingManager.setSlashPolicy(REASON_EVIDENCE, {
        ticketPenalty: ethers.parseUnits("10", 6),
        licensePenalty: ethers.parseEther("50"),
        requiresProof: false,
        proofVerifier: ethers.ZeroAddress,
        banNode: false,
        appealWindow: 1, // Minimum appeal window (1 second)
        enabled: true,
        affectsCommittee: true,
        failureReason: 4,
      });
      const SLASHER_ROLE = await slashingManager.SLASHER_ROLE();
      await slashingManager.grantRole(SLASHER_ROLE, await owner.getAddress());

      await makeRequest(1); // Small: M=2, N=3
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
      ]);

      // Step 1: Lane A slash op1 — still viable (3→2 active, >= M=2)
      const laneAProof = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
        await slashingManager.getAddress(),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        laneAProof,
      );

      // Step 2: Lane A cannot slash op2 (only op3 can vote, 1 < M=2).
      // Lane B (SLASHER_ROLE) is required for the final expulsion.
      const nextProposalId = await slashingManager.totalProposals();
      await slashingManager.proposeSlashEvidence(
        0,
        await operator2.getAddress(),
        REASON_EVIDENCE,
        ethers.toUtf8Bytes("evidence-data"),
      );

      // Wait for appeal window to pass, then execute
      await time.increase(2);
      const tx = await slashingManager.executeSlash(nextProposalId);

      await expect(tx).to.emit(enclave, "E3Failed");

      // Should emit CommitteeViabilityUpdated(viable=false)
      // activeCount drops to 1, which is < M=2
      await expect(tx)
        .to.emit(registry, "CommitteeViabilityUpdated")
        .withArgs(0, 1, 2, false);

      const stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(6); // Failed
    });

    it("should not fail E3 twice on multiple sub-threshold expulsions", async function () {
      const {
        enclave,
        slashingManager,
        operator1,
        operator2,
        operator3,
        operator4,
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);
      await setupOperator(operator3);
      await setupOperator(operator4);

      await makeRequest(2); // Medium: M=2, N=4
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
        operator4,
      ]);

      // Expel operator1 — still viable (3 >= 2)
      const proof1 = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
        await slashingManager.getAddress(),
        0,
        31337,
        ethers.keccak256(ethers.toUtf8Bytes("expel-op1")),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        proof1,
      );

      // Expel operator2 — still viable (2 >= 2)
      const proof2 = await signAndEncodeAttestation(
        [operator3, operator4],
        0,
        await operator2.getAddress(),
        await slashingManager.getAddress(),
        0,
        31337,
        ethers.keccak256(ethers.toUtf8Bytes("expel-op2")),
      );
      await slashingManager.proposeSlash(
        0,
        await operator2.getAddress(),
        proof2,
      );

      let stage = await enclave.getE3Stage(0);
      expect(stage).to.not.equal(6); // Not failed yet

      // At this point only operator3 and operator4 are active (2 == M=2).
      // Lane A cannot slash further: to expel operator3, we need M=2 non-accused
      // active voters, but only operator4 is available (1 < 2).
      // This proves Lane A naturally stops at M active members.
      // Lane B (SLASHER_ROLE) is required for the final slash.
      // TODO: See GitHub issue — "Lane B governance flow for M-threshold slashing"
      await expect(
        slashingManager.proposeSlash(
          0,
          await operator3.getAddress(),
          await signAndEncodeAttestation(
            [operator4],
            0,
            await operator3.getAddress(),
            await slashingManager.getAddress(),
            0,
            31337,
            ethers.keccak256(ethers.toUtf8Bytes("expel-op3")),
          ),
        ),
      ).to.be.revertedWithCustomError(
        slashingManager,
        "InsufficientAttestations",
      );

      // E3 stage should still NOT be Failed — only 2 active, which equals M
      const stageAfter = await enclave.getE3Stage(0);
      expect(stageAfter).to.not.equal(6);
    });
  });

  describe("slash execution events", function () {
    it("should emit SlashExecuted on proof-based committee slash", async function () {
      const {
        slashingManager,
        operator1,
        operator2,
        operator3,
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);
      await setupOperator(operator3);

      await makeRequest(1);
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
      ]);

      const proof = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
        await slashingManager.getAddress(),
      );
      const op1Addr = await operator1.getAddress();
      const tx = await slashingManager.proposeSlash(0, op1Addr, proof);

      await expect(tx).to.emit(slashingManager, "SlashExecuted").withArgs(
        0, // proposalId
        0, // e3Id
        op1Addr,
        REASON_PT_0,
        ethers.parseUnits("10", 6), // ticketPenalty
        ethers.parseEther("50"), // licensePenalty
        true, // executed
        0, // lane: LaneA (attestation/proof-based via proposeSlash)
      );
    });
  });
});
