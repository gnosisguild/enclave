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

  const REASON_BAD_DKG = ethers.keccak256(
    ethers.toUtf8Bytes("E3_BAD_DKG_PROOF"),
  );
  const REASON_BAD_DECRYPTION = ethers.keccak256(
    ethers.toUtf8Bytes("E3_BAD_DECRYPTION_PROOF"),
  );

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
    dkgWindow: ONE_DAY,
    computeWindow: THREE_DAYS,
    decryptionWindow: ONE_DAY,
  };

  // Must match the PROOF_PAYLOAD_TYPEHASH in SlashingManager.sol
  const PROOF_PAYLOAD_TYPEHASH = ethers.keccak256(
    ethers.toUtf8Bytes(
      "ProofPayload(uint256 chainId,uint256 e3Id,uint256 proofType,bytes zkProof,bytes publicSignals)",
    ),
  );

  /**
   * Helper to create a signed proof evidence bundle.
   * The operator signs the proof payload (matching SlashingManager._verifyProofEvidence),
   * then the evidence is encoded in the 6-field format expected by proposeSlash().
   */
  async function signAndEncodeProof(
    signer: Signer,
    e3Id: number,
    verifierAddress: string,
    zkProof: string = "0x1234",
    publicInputs: string[] = [ethers.ZeroHash],
    chainId: number = 31337,
    proofType: number = 0,
  ): Promise<string> {
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
    return abiCoder.encode(
      ["bytes", "bytes32[]", "bytes", "uint256", "uint256", "address"],
      [zkProof, publicInputs, signature, chainId, proofType, verifierAddress],
    );
  }

  const setup = async () => {
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

    // Deploy tokens
    const usdcContract = await ignition.deploy(MockStableTokenModule, {
      parameters: { MockUSDC: { initialSupply: 10000000 } },
    });
    const usdcToken = MockUSDCFactory.connect(
      await usdcContract.mockUSDC.getAddress(),
      owner,
    );

    const enclTokenContract = await ignition.deploy(EnclaveTokenModule, {
      parameters: { EnclaveToken: { owner: ownerAddress } },
    });
    const enclToken = EnclaveTokenFactory.connect(
      await enclTokenContract.enclaveToken.getAddress(),
      owner,
    );
    await enclToken.setTransferRestriction(false);

    const ticketTokenContract = await ignition.deploy(
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

    const mockVerifierContract = await ignition.deploy(
      MockCircuitVerifierModule,
    );
    const mockVerifier = MockCircuitVerifierFactory.connect(
      await mockVerifierContract.mockCircuitVerifier.getAddress(),
      owner,
    );

    // Deploy slashing manager
    const slashingManagerContract = await ignition.deploy(
      SlashingManagerModule,
      {
        parameters: {
          SlashingManager: {
            admin: ownerAddress,
            bondingRegistry: addressOne,
            ciphernodeRegistry: addressOne,
            enclave: addressOne,
          },
        },
      },
    );
    const slashingManager = SlashingManagerFactory.connect(
      await slashingManagerContract.slashingManager.getAddress(),
      owner,
    );

    // Deploy bonding registry
    const bondingRegistryContract = await ignition.deploy(
      BondingRegistryModule,
      {
        parameters: {
          BondingRegistry: {
            owner: ownerAddress,
            ticketToken:
              await ticketTokenContract.enclaveTicketToken.getAddress(),
            licenseToken: await enclToken.getAddress(),
            registry: addressOne,
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
      await bondingRegistryContract.bondingRegistry.getAddress(),
      owner,
    );

    // Deploy Enclave
    const enclaveContract = await ignition.deploy(EnclaveModule, {
      parameters: {
        Enclave: {
          params: encodedE3ProgramParams,
          owner: ownerAddress,
          maxDuration: THIRTY_DAYS,
          registry: addressOne,
          e3RefundManager: addressOne,
          bondingRegistry: await bondingRegistry.getAddress(),
          feeToken: await usdcToken.getAddress(),
          timeoutConfig: defaultTimeoutConfig,
        },
      },
    });
    const enclaveAddress = await enclaveContract.enclave.getAddress();
    const enclave = EnclaveFactory.connect(enclaveAddress, owner);

    // Deploy CiphernodeRegistry
    const ciphernodeRegistryContract = await ignition.deploy(
      CiphernodeRegistryModule,
      {
        parameters: {
          CiphernodeRegistry: {
            enclaveAddress: enclaveAddress,
            owner: ownerAddress,
            submissionWindow: SORTITION_SUBMISSION_WINDOW,
          },
        },
      },
    );
    const registryAddress =
      await ciphernodeRegistryContract.cipherNodeRegistry.getAddress();
    const registry = CiphernodeRegistryOwnableFactory.connect(
      registryAddress,
      owner,
    );

    // Deploy mock E3 program
    const e3ProgramContract = await ignition.deploy(MockE3ProgramModule, {
      parameters: {
        MockE3Program: {
          encryptionSchemeId: encryptionSchemeId,
        },
      },
    });
    const e3Program = MockE3ProgramFactory.connect(
      await e3ProgramContract.mockE3Program.getAddress(),
      owner,
    );

    // Deploy mock decryption verifier
    const decryptionVerifierContract = await ignition.deploy(
      MockDecryptionVerifierModule,
    );
    const decryptionVerifier = MockDecryptionVerifierFactory.connect(
      await decryptionVerifierContract.mockDecryptionVerifier.getAddress(),
      owner,
    );

    // Wire everything together
    await enclave.setCiphernodeRegistry(registryAddress);
    await enclave.enableE3Program(await e3Program.getAddress());
    await enclave.setDecryptionVerifier(
      encryptionSchemeId,
      await decryptionVerifier.getAddress(),
    );
    await enclave.setSlashingManager(await slashingManager.getAddress());

    await bondingRegistry.setRewardDistributor(enclaveAddress);
    await bondingRegistry.setRegistry(registryAddress);
    await bondingRegistry.setSlashingManager(
      await slashingManager.getAddress(),
    );

    await slashingManager.setBondingRegistry(
      await bondingRegistry.getAddress(),
    );
    await slashingManager.setCiphernodeRegistry(registryAddress);
    await slashingManager.setEnclave(enclaveAddress);

    await registry.setBondingRegistry(await bondingRegistry.getAddress());
    await registry.setSlashingManager(await slashingManager.getAddress());

    await ticketTokenContract.enclaveTicketToken.setRegistry(
      await bondingRegistry.getAddress(),
    );

    // Mint tokens to requester for E3 requests
    await usdcToken.mint(requesterAddress, ethers.parseUnits("100000", 6));

    // Helper: setup an operator (bond license, register, add tickets)
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

      const ticketTokenAddress = await bondingRegistry.ticketToken();
      const ticketAmount = ethers.parseUnits("100", 6);
      await usdcToken
        .connect(operator)
        .approve(ticketTokenAddress, ticketAmount);
      await bondingRegistry.connect(operator).addTicketBalance(ticketAmount);
    }

    // Helper: make an E3 request
    async function makeRequest(threshold: [number, number] = [2, 3]) {
      const startTime = (await time.latest()) + 100;
      const requestParams = {
        threshold: threshold,
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

    // Helper: finalize a committee after sortition
    async function finalizeCommitteeWithOperators(
      e3Id: number,
      operators: Signer[],
    ) {
      for (const op of operators) {
        await registry.connect(op).submitTicket(e3Id, 1);
      }
      await time.increase(SORTITION_SUBMISSION_WINDOW + 1);
      await registry.finalizeCommittee(e3Id);

      // Publish the committee key so getCommitteeNodes works
      const nodes = await Promise.all(operators.map((op) => op.getAddress()));
      const publicKey = ethers.toUtf8Bytes("fake-public-key");
      const publicKeyHash = ethers.keccak256(publicKey);
      await registry.publishCommittee(e3Id, nodes, publicKey, publicKeyHash);
    }

    // Set up committee-affecting slash policy
    // MockCircuitVerifier returns false by default → proof invalid → fault confirmed
    const committeeSlashPolicy = {
      ticketPenalty: ethers.parseUnits("10", 6),
      licensePenalty: ethers.parseEther("50"),
      requiresProof: true,
      proofVerifier: await mockVerifier.getAddress(),
      banNode: false,
      appealWindow: 0,
      enabled: true,
      affectsCommittee: true,
      failureReason: 4, // FailureReason.DKGInvalidShares
    };
    await slashingManager.setSlashPolicy(REASON_BAD_DKG, committeeSlashPolicy);

    const decryptionSlashPolicy = {
      ticketPenalty: ethers.parseUnits("10", 6),
      licensePenalty: ethers.parseEther("50"),
      requiresProof: true,
      proofVerifier: await mockVerifier.getAddress(),
      banNode: false,
      appealWindow: 0,
      enabled: true,
      affectsCommittee: true,
      failureReason: 11, // FailureReason.DecryptionInvalidShares
    };
    await slashingManager.setSlashPolicy(
      REASON_BAD_DECRYPTION,
      decryptionSlashPolicy,
    );

    return {
      enclave,
      registry,
      slashingManager,
      bondingRegistry,
      mockVerifier,
      usdcToken,
      enclToken,
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
        mockVerifier,
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
      expect(await registry.getActiveCommitteeCount(0)).to.equal(3);

      // Submit slash proposal — MockCircuitVerifier returns false by default
      // so fault is confirmed and slash is auto-executed
      const proof = await signAndEncodeProof(
        operator1,
        0,
        await mockVerifier.getAddress(),
      );
      const tx = await slashingManager.proposeSlash(
        0,
        op1Address,
        REASON_BAD_DKG,
        proof,
      );

      // Should emit CommitteeMemberExpelled
      await expect(tx)
        .to.emit(registry, "CommitteeMemberExpelled")
        .withArgs(0, op1Address, REASON_BAD_DKG, 2);

      // Should emit CommitteeViabilityUpdated
      await expect(tx)
        .to.emit(registry, "CommitteeViabilityUpdated")
        .withArgs(0, 2, 2, true); // 2 >= 2 → viable

      // Verify member is no longer active
      expect(await registry.isCommitteeMemberActive(0, op1Address)).to.be.false;
      expect(await registry.getActiveCommitteeCount(0)).to.equal(2);
    });

    it("should keep E3 alive when active members >= threshold", async function () {
      const {
        enclave,
        registry,
        slashingManager,
        mockVerifier,
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
      const proof = await signAndEncodeProof(
        operator1,
        0,
        await mockVerifier.getAddress(),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_DKG,
        proof,
      );

      // E3 should NOT be failed — stage should still be Requested (1)
      // or whatever stage it was at, not Failed
      const stage = await enclave.getE3Stage(0);
      expect(stage).to.not.equal(6); // 6 = E3Stage.Failed

      // Active committee still has enough members
      expect(await registry.getActiveCommitteeCount(0)).to.equal(2);
      const threshold = await registry.getCommitteeThreshold(0);
      expect(threshold[0]).to.equal(2); // M=2
    });

    it("should fail E3 when active members drop below threshold", async function () {
      const {
        enclave,
        slashingManager,
        mockVerifier,
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

      // Slash first member — 3 → 2 active, still >= 2
      const proof1 = await signAndEncodeProof(
        operator1,
        0,
        await mockVerifier.getAddress(),
        "0x1111",
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_DKG,
        proof1,
      );

      let stage = await enclave.getE3Stage(0);
      expect(stage).to.not.equal(6); // Not failed yet

      // Slash second member — 2 → 1 active, below threshold M=2
      const proof2 = await signAndEncodeProof(
        operator2,
        0,
        await mockVerifier.getAddress(),
        "0x2222",
      );
      const tx = await slashingManager.proposeSlash(
        0,
        await operator2.getAddress(),
        REASON_BAD_DKG,
        proof2,
      );

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
        mockVerifier,
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
      const proof1 = await signAndEncodeProof(
        operator1,
        0,
        await mockVerifier.getAddress(),
        "0xaaaa",
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_DKG,
        proof1,
      );
      expect(await registry.getActiveCommitteeCount(0)).to.equal(2);

      // Slash operator1 again with different proof (different evidence key)
      const proof2 = await signAndEncodeProof(
        operator1,
        0,
        await mockVerifier.getAddress(),
        "0xbbbb",
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_DKG,
        proof2,
      );

      // Active count should still be 2 (idempotent expulsion)
      expect(await registry.getActiveCommitteeCount(0)).to.equal(2);
    });

    it("should exclude expelled members from getActiveCommitteeNodes", async function () {
      const {
        registry,
        slashingManager,
        mockVerifier,
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
      const proof = await signAndEncodeProof(
        operator1,
        0,
        await mockVerifier.getAddress(),
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_DKG,
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
        mockVerifier,
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

      expect(await registry.getActiveCommitteeCount(0)).to.equal(4);

      // Expel 2 out of 4 — still have 2 >= M=2
      const proof1 = await signAndEncodeProof(
        operator1,
        0,
        await mockVerifier.getAddress(),
        "0x1111",
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_DKG,
        proof1,
      );
      expect(await registry.getActiveCommitteeCount(0)).to.equal(3);

      const proof2 = await signAndEncodeProof(
        operator2,
        0,
        await mockVerifier.getAddress(),
        "0x2222",
      );
      await slashingManager.proposeSlash(
        0,
        await operator2.getAddress(),
        REASON_BAD_DKG,
        proof2,
      );
      expect(await registry.getActiveCommitteeCount(0)).to.equal(2);

      // E3 should NOT be failed
      const stage = await enclave.getE3Stage(0);
      expect(stage).to.not.equal(6);
    });
  });

  describe("E3 fails below threshold", function () {
    it("should fail E3 exactly at the threshold breach", async function () {
      const {
        enclave,
        registry,
        slashingManager,
        mockVerifier,
        operator1,
        operator2,
        setupOperator,
        makeRequest,
        finalizeCommitteeWithOperators,
      } = await loadFixture(setup);

      await setupOperator(operator1);
      await setupOperator(operator2);

      await makeRequest([2, 2]); // M=2, N=2 — no room for error
      await finalizeCommitteeWithOperators(0, [operator1, operator2]);

      // Expel one member: 2 → 1 < M=2 → E3 fails immediately
      const proof = await signAndEncodeProof(
        operator1,
        0,
        await mockVerifier.getAddress(),
      );
      const tx = await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_DKG,
        proof,
      );

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
        mockVerifier,
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

      // Expel operator1 — still viable (2 >= 2)
      const proof1 = await signAndEncodeProof(
        operator1,
        0,
        await mockVerifier.getAddress(),
        "0x1111",
      );
      await slashingManager.proposeSlash(
        0,
        await operator1.getAddress(),
        REASON_BAD_DKG,
        proof1,
      );

      // Expel operator2 — now below threshold (1 < 2), E3 fails
      const proof2 = await signAndEncodeProof(
        operator2,
        0,
        await mockVerifier.getAddress(),
        "0x2222",
      );
      await slashingManager.proposeSlash(
        0,
        await operator2.getAddress(),
        REASON_BAD_DKG,
        proof2,
      );

      // E3 is now Failed
      const stage = await enclave.getE3Stage(0);
      expect(stage).to.equal(6);

      // Try to expel operator3 — E3 already failed, but onE3Failed is wrapped
      // in try-catch so financial penalties are still applied
      const proof3 = await signAndEncodeProof(
        operator3,
        0,
        await mockVerifier.getAddress(),
        "0x3333",
      );

      // The third slash should succeed — penalties are applied even though E3 is already Failed.
      // The onE3Failed call silently fails (try-catch) since E3 is already in Failed state.
      await expect(
        slashingManager.proposeSlash(
          0,
          await operator3.getAddress(),
          REASON_BAD_DKG,
          proof3,
        ),
      ).to.emit(slashingManager, "SlashExecuted");

      // E3 stage should still be Failed
      const stageAfter = await enclave.getE3Stage(0);
      expect(stageAfter).to.equal(6);
    });
  });

  describe("slash execution events", function () {
    it("should emit SlashExecuted on proof-based committee slash", async function () {
      const {
        slashingManager,
        mockVerifier,
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

      const proof = await signAndEncodeProof(
        operator1,
        0,
        await mockVerifier.getAddress(),
      );
      const op1Addr = await operator1.getAddress();
      const tx = await slashingManager.proposeSlash(
        0,
        op1Addr,
        REASON_BAD_DKG,
        proof,
      );

      await expect(tx).to.emit(slashingManager, "SlashExecuted").withArgs(
        0, // proposalId
        0, // e3Id
        op1Addr,
        REASON_BAD_DKG,
        ethers.parseUnits("10", 6), // ticketPenalty
        ethers.parseEther("50"), // licensePenalty
        true, // executed
      );
    });
  });
});
