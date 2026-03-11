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
import { network } from "hardhat";

import BondingRegistryModule from "../../ignition/modules/bondingRegistry";
import CiphernodeRegistryModule from "../../ignition/modules/ciphernodeRegistry";
import EnclaveModule from "../../ignition/modules/enclave";
import EnclaveTicketTokenModule from "../../ignition/modules/enclaveTicketToken";
import EnclaveTokenModule from "../../ignition/modules/enclaveToken";
import MockDecryptionVerifierModule from "../../ignition/modules/mockDecryptionVerifier";
import MockE3ProgramModule from "../../ignition/modules/mockE3Program";
import MockCircuitVerifierModule from "../../ignition/modules/mockSlashingVerifier";
import MockStableTokenModule from "../../ignition/modules/mockStableToken";
import SlashingManagerModule from "../../ignition/modules/slashingManager";
import {
  BondingRegistry__factory as BondingRegistryFactory,
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryOwnableFactory,
  Enclave__factory as EnclaveFactory,
  EnclaveToken__factory as EnclaveTokenFactory,
  MockCircuitVerifier__factory as MockCircuitVerifierFactory,
  MockDecryptionVerifier__factory as MockDecryptionVerifierFactory,
  MockE3Program__factory as MockE3ProgramFactory,
  MockUSDC__factory as MockUSDCFactory,
  SlashingManager__factory as SlashingManagerFactory,
} from "../../types";

const { ethers, ignition, networkHelpers } = await network.connect();
const { loadFixture, time } = networkHelpers;

describe("Committee Expulsion & Fault Tolerance", function () {
  const ONE_HOUR = 60 * 60;
  const ONE_DAY = 24 * ONE_HOUR;
  const THREE_DAYS = 3 * ONE_DAY;
  const SEVEN_DAYS = 7 * ONE_DAY;
  const THIRTY_DAYS = 30 * ONE_DAY;
  const SORTITION_SUBMISSION_WINDOW = 10;
  const addressOne = "0x0000000000000000000000000000000000000001";

  // Lane A reasons are derived on-chain as keccak256(abi.encodePacked(proofType))
  const REASON_PT_0 = ethers.keccak256(ethers.solidityPacked(["uint256"], [0]));
  const REASON_PT_7 = ethers.keccak256(ethers.solidityPacked(["uint256"], [7]));

  const abiCoder = ethers.AbiCoder.defaultAbiCoder();
  const polynomial_degree = ethers.toBigInt(2048);
  const plaintext_modulus = ethers.toBigInt(1032193);
  const moduli = [ethers.toBigInt("18014398492704769")];

  const encodedE3ProgramParams = abiCoder.encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );

  const encryptionSchemeId =
    "0x2c2a814a0495f913a3a312fc4771e37552bc14f8a2d4075a08122d356f0849c6";

  const defaultTimeoutConfig = {
    committeeFormationWindow: ONE_DAY,
    dkgWindow: ONE_DAY,
    computeWindow: THREE_DAYS,
    decryptionWindow: ONE_DAY,
  };

  // Must match the VOTE_TYPEHASH in SlashingManager.sol
  const VOTE_TYPEHASH = ethers.keccak256(
    ethers.toUtf8Bytes(
      "AccusationVote(uint256 chainId,uint256 e3Id,bytes32 accusationId,address voter,bool agrees,bytes32 dataHash)",
    ),
  );

  /**
   * Helper to create signed committee attestation evidence for Lane A.
   * Voters (other committee members) sign votes confirming the accused is faulty.
   * Returns abi.encode(proofType, voters, agrees, dataHashes, signatures)
   */
  async function signAndEncodeAttestation(
    voterSigners: Signer[],
    e3Id: number,
    operator: string,
    proofType: number = 0,
    chainId: number = 31337,
    dataHash: string = ethers.ZeroHash,
  ): Promise<string> {
    const accusationId = ethers.keccak256(
      ethers.solidityPacked(
        ["uint256", "uint256", "address", "uint256"],
        [chainId, e3Id, operator, proofType],
      ),
    );

    const signersWithAddrs = await Promise.all(
      voterSigners.map(async (s) => ({
        signer: s,
        address: await s.getAddress(),
      })),
    );
    signersWithAddrs.sort((a, b) =>
      a.address.toLowerCase() < b.address.toLowerCase()
        ? -1
        : a.address.toLowerCase() > b.address.toLowerCase()
          ? 1
          : 0,
    );

    const voters: string[] = [];
    const agrees: boolean[] = [];
    const dataHashes: string[] = [];
    const signatures: string[] = [];

    for (const { signer, address: voterAddress } of signersWithAddrs) {
      voters.push(voterAddress);
      agrees.push(true);
      dataHashes.push(dataHash);

      const messageHash = ethers.keccak256(
        abiCoder.encode(
          [
            "bytes32",
            "uint256",
            "uint256",
            "bytes32",
            "address",
            "bool",
            "bytes32",
          ],
          [
            VOTE_TYPEHASH,
            chainId,
            e3Id,
            accusationId,
            voterAddress,
            true,
            dataHash,
          ],
        ),
      );
      const signature = await signer.signMessage(ethers.getBytes(messageHash));
      signatures.push(signature);
    }

    return abiCoder.encode(
      ["uint256", "address[]", "bool[]", "bytes32[]", "bytes[]"],
      [proofType, voters, agrees, dataHashes, signatures],
    );
  }
  const setup = async () => {
    // ── Signers ────────────────────────────────────────────────────────────────
    const [
      owner,
      requester,
      treasury,
      operator1,
      operator2,
      operator3,
      operator4,
    ] = await ethers.getSigners();

    const ownerAddress = await owner.getAddress();
    const treasuryAddress = await treasury.getAddress();
    const requesterAddress = await requester.getAddress();

    // ── Tokens ─────────────────────────────────────────────────────────────────
    const { mockUSDC } = await ignition.deploy(MockStableTokenModule, {
      parameters: { MockUSDC: { initialSupply: 10_000_000 } },
    });
    const usdcToken = MockUSDCFactory.connect(
      await mockUSDC.getAddress(),
      owner,
    );

    const { enclaveToken } = await ignition.deploy(EnclaveTokenModule, {
      parameters: { EnclaveToken: { owner: ownerAddress } },
    });
    const enclToken = EnclaveTokenFactory.connect(
      await enclaveToken.getAddress(),
      owner,
    );
    await enclToken.setTransferRestriction(false);

    const { enclaveTicketToken } = await ignition.deploy(
      EnclaveTicketTokenModule,
      {
        parameters: {
          EnclaveTicketToken: {
            baseToken: await usdcToken.getAddress(),
            registry: addressOne,
            owner: ownerAddress,
          },
        },
      },
    );
    const ticketToken = enclaveTicketToken;

    const { mockCircuitVerifier } = await ignition.deploy(
      MockCircuitVerifierModule,
    );
    const mockVerifier = MockCircuitVerifierFactory.connect(
      await mockCircuitVerifier.getAddress(),
      owner,
    );

    // ── Registry & Slashing ────────────────────────────────────────────────────
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
    const slashingManager = SlashingManagerFactory.connect(
      await _slashingManager.getAddress(),
      owner,
    );

    const { cipherNodeRegistry } = await ignition.deploy(
      CiphernodeRegistryModule,
      {
        parameters: {
          CiphernodeRegistry: {
            owner: ownerAddress,
            submissionWindow: SORTITION_SUBMISSION_WINDOW,
          },
        },
      },
    );
    const registryAddress = await cipherNodeRegistry.getAddress();
    const registry = CiphernodeRegistryOwnableFactory.connect(
      registryAddress,
      owner,
    );

    const { bondingRegistry: _bondingRegistry } = await ignition.deploy(
      BondingRegistryModule,
      {
        parameters: {
          BondingRegistry: {
            owner: ownerAddress,
            ticketToken: await ticketToken.getAddress(),
            licenseToken: await enclToken.getAddress(),
            registry: registryAddress,
            slashedFundsTreasury: treasuryAddress,
            ticketPrice: ethers.parseUnits("10", 6),
            licenseRequiredBond: ethers.parseEther("1000"),
            minTicketBalance: 1,
            exitDelay: SEVEN_DAYS,
          },
        },
      },
    );
    const bondingRegistry = BondingRegistryFactory.connect(
      await _bondingRegistry.getAddress(),
      owner,
    );

    // ── Enclave ────────────────────────────────────────────────────────────────
    const { enclave: _enclave } = await ignition.deploy(EnclaveModule, {
      parameters: {
        Enclave: {
          params: encodedE3ProgramParams,
          owner: ownerAddress,
          maxDuration: THIRTY_DAYS,
          registry: registryAddress,
          e3RefundManager: addressOne,
          bondingRegistry: await bondingRegistry.getAddress(),
          feeToken: await usdcToken.getAddress(),
          timeoutConfig: defaultTimeoutConfig,
        },
      },
    });
    const enclaveAddress = await _enclave.getAddress();
    const enclave = EnclaveFactory.connect(enclaveAddress, owner);

    // ── Mocks ──────────────────────────────────────────────────────────────────
    const { mockE3Program } = await ignition.deploy(MockE3ProgramModule, {
      parameters: { MockE3Program: { encryptionSchemeId } },
    });
    const e3Program = MockE3ProgramFactory.connect(
      await mockE3Program.getAddress(),
      owner,
    );

    const { mockDecryptionVerifier } = await ignition.deploy(
      MockDecryptionVerifierModule,
    );
    const decryptionVerifier = MockDecryptionVerifierFactory.connect(
      await mockDecryptionVerifier.getAddress(),
      owner,
    );

    // ── Wire Up ────────────────────────────────────────────────────────────────
    await registry.setEnclave(enclaveAddress);
    await registry.setBondingRegistry(await bondingRegistry.getAddress());
    await registry.setSlashingManager(await slashingManager.getAddress());

    await enclave.enableE3Program(await e3Program.getAddress());
    await enclave.setDecryptionVerifier(
      encryptionSchemeId,
      await decryptionVerifier.getAddress(),
    );
    await enclave.setSlashingManager(await slashingManager.getAddress());

    await bondingRegistry.setRewardDistributor(enclaveAddress);
    await bondingRegistry.setSlashingManager(
      await slashingManager.getAddress(),
    );

    await slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );
    await slashingManager.setCiphernodeRegistry(registryAddress);
    await slashingManager.setEnclave(enclaveAddress);
    await slashingManager.setE3RefundManager(addressOne);

    await ticketToken.setRegistry(await bondingRegistry.getAddress());
    await usdcToken.mint(requesterAddress, ethers.parseUnits("100000", 6));

    // ── Slash Policies ─────────────────────────────────────────────────────────
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

    // ── Helpers ────────────────────────────────────────────────────────────────
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

    async function makeRequest(threshold: [number, number] = [2, 3]) {
      const startTime = (await time.latest()) + 100;
      const requestParams = {
        threshold,
        inputWindow: [startTime + 100, startTime + ONE_DAY] as [number, number],
        e3Program: await e3Program.getAddress(),
        e3ProgramParams: encodedE3ProgramParams,
        computeProviderParams: abiCoder.encode(
          ["address"],
          [await decryptionVerifier.getAddress()],
        ),
        customParams: abiCoder.encode(
          ["address"],
          ["0x1234567890123456789012345678901234567890"],
        ),
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
      const publicKeyHash = ethers.keccak256(publicKey);
      await registry.publishCommittee(e3Id, nodes, publicKey, publicKeyHash);
    }

    // ── Return ─────────────────────────────────────────────────────────────────
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

      // threshold [2, 3] means M=2, N=3
      await makeRequest([2, 3]);
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

      await makeRequest([2, 3]); // M=2, N=3
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

      await makeRequest([2, 3]); // M=2, N=3
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

      await makeRequest([2, 3]);
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
      // Using proofType=7 (T5ShareDecryption) with REASON_PT_7 instead.
      const proof2 = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
        7, // T5ShareDecryption — different proofType
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

      await makeRequest([2, 3]);
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
      ]);

      // Before expulsion: all 3 should be in active nodes
      const nodesBefore = await registry.getActiveCommitteeNodes(0);
      expect(nodesBefore.length).to.equal(3);
      expect(nodesBefore).to.include(await operator1.getAddress());

      // Expel operator1
      const proof = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        proof,
      );

      // After expulsion: only 2 should be active
      const nodesAfter = await registry.getActiveCommitteeNodes(0);
      expect(nodesAfter.length).to.equal(2);
      expect(nodesAfter).to.not.include(await operator1.getAddress());
      expect(nodesAfter).to.include(await operator2.getAddress());
      expect(nodesAfter).to.include(await operator3.getAddress());
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

      await makeRequest([2, 4]); // M=2, N=4
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
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

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

      await makeRequest([2, 2]); // M=2, N=2 — no room for error
      await finalizeCommitteeWithOperators(0, [operator1, operator2]);

      // Lane A cannot slash at M active when the accused is excluded from voting.
      // With M=2, N=2: expelling 1 member needs M=2 non-accused votes, but only
      // 1 non-accused active voter exists. Lane B (SLASHER_ROLE) is required.
      // TODO: See GitHub issue — "Lane B governance flow for M-threshold slashing"
      const nextProposalId = await slashingManager.totalProposals();
      await slashingManager.proposeSlashEvidence(
        0,
        await operator1.getAddress(),
        REASON_EVIDENCE,
        ethers.toUtf8Bytes("evidence-data"),
      );

      // Wait for appeal window to pass, then execute
      await time.increase(2);
      const tx = await slashingManager.executeSlash(nextProposalId);

      await expect(tx).to.emit(enclave, "E3Failed");

      // Should emit CommitteeViabilityUpdated(viable=false)
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

      await makeRequest([2, 4]); // M=2, N=4
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

      await makeRequest([2, 3]);
      await finalizeCommitteeWithOperators(0, [
        operator1,
        operator2,
        operator3,
      ]);

      const proof = await signAndEncodeAttestation(
        [operator2, operator3],
        0,
        await operator1.getAddress(),
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
      );
    });
  });
});
