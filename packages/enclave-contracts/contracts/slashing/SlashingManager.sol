// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import {
    AccessControl
} from "@openzeppelin/contracts/access/AccessControl.sol";
import { ECDSA } from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {
    MessageHashUtils
} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";
import { ISlashingManager } from "../interfaces/ISlashingManager.sol";
import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";
import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IEnclave } from "../interfaces/IEnclave.sol";
import { IE3RefundManager } from "../interfaces/IE3RefundManager.sol";

/**
 * @title SlashingManager
 * @notice Implementation of slashing management with two-lane architecture:
 *         Lane A (proof-based): permissionless, atomic propose+execute, no appeals
 *         Lane B (evidence-based): SLASHER_ROLE required, appeal window, separate execute
 * @dev Role-based access control for slashers and governance with configurable slash policies.
 *      Integrates with CiphernodeRegistry for committee expulsion and Enclave for E3 failure.
 */
contract SlashingManager is ISlashingManager, AccessControl {
    // ======================
    // Constants & Roles
    // ======================

    /// @notice Role identifier for accounts authorized to propose evidence-based slashes
    bytes32 public constant SLASHER_ROLE = keccak256("SLASHER_ROLE");

    /// @notice Role identifier for governance accounts that can configure policies, resolve appeals, and manage bans
    bytes32 public constant GOVERNANCE_ROLE = keccak256("GOVERNANCE_ROLE");

    // ======================
    // Storage
    // ======================

    /// @notice Reference to the bonding registry contract where slash penalties are executed
    IBondingRegistry public bondingRegistry;

    /// @notice Reference to the ciphernode registry for committee expulsion
    ICiphernodeRegistry public ciphernodeRegistry;

    /// @notice Reference to the Enclave contract for E3 failure signaling
    IEnclave public enclave;

    /// @notice Reference to the E3 Refund Manager for routing slashed funds
    IE3RefundManager public e3RefundManager;

    /// @notice Mapping from slash reason hash to its configured policy
    mapping(bytes32 reason => SlashPolicy policy) public slashPolicies;

    /// @notice Internal storage for all slash proposals indexed by proposal ID
    mapping(uint256 proposalId => SlashProposal proposal) internal _proposals;

    /// @notice Counter for total number of slash proposals ever created
    uint256 public totalProposals;

    /// @notice Mapping tracking which nodes are currently banned from the network
    mapping(address node => bool banned) public banned;

    /// @notice Evidence replay protection: tracks consumed evidence keys
    /// @dev Key is keccak256(abi.encode(e3Id, operator, keccak256(proof))) — reason-independent
    ///      to prevent the same proof/evidence from being used to slash under multiple reasons.
    mapping(bytes32 evidenceKey => bool consumed) public evidenceConsumed;

    // ======================
    // Constants
    // ======================

    /// @notice EIP-712 style typehash for the operator's signed proof payload.
    /// @dev Must match `ProofPayload::typehash()` in `crates/events/src/enclave_event/signed_proof.rs`.
    ///      Prevents cross-chain, cross-E3, and cross-proof-type replay of signed proofs.
    bytes32 public constant PROOF_PAYLOAD_TYPEHASH =
        keccak256(
            "ProofPayload(uint256 chainId,uint256 e3Id,uint256 proofType,bytes zkProof,bytes publicSignals)"
        );

    /// @notice EIP-712 style typehash for committee attestation votes.
    /// @dev Must match `AccusationManager::vote_digest()` in `crates/zk-prover/src/actors/accusation_manager.rs`.
    ///      Includes chainId to prevent cross-chain replay and dataHash for equivocation detection.
    bytes32 public constant VOTE_TYPEHASH =
        keccak256(
            "AccusationVote(uint256 chainId,uint256 e3Id,"
            "bytes32 accusationId,address voter,"
            "bool agrees,bytes32 dataHash)"
        );

    // ======================
    // Modifiers
    // ======================

    /// @notice Restricts function access to accounts with SLASHER_ROLE
    modifier onlySlasher() {
        if (!hasRole(SLASHER_ROLE, msg.sender)) revert Unauthorized();
        _;
    }

    /// @notice Restricts function access to accounts with GOVERNANCE_ROLE
    modifier onlyGovernance() {
        if (!hasRole(GOVERNANCE_ROLE, msg.sender)) revert Unauthorized();
        _;
    }

    // ======================
    // Constructor
    // ======================

    /**
     * @notice Initializes the SlashingManager contract
     * @param admin Address to receive DEFAULT_ADMIN_ROLE and GOVERNANCE_ROLE
     */
    constructor(address admin) {
        require(admin != address(0), ZeroAddress());
        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(GOVERNANCE_ROLE, admin);
    }

    // ======================
    // View Functions
    // ======================

    /// @inheritdoc ISlashingManager
    function getSlashPolicy(
        bytes32 reason
    ) external view returns (SlashPolicy memory) {
        return slashPolicies[reason];
    }

    /// @inheritdoc ISlashingManager
    function getSlashProposal(
        uint256 proposalId
    ) external view returns (SlashProposal memory) {
        require(proposalId < totalProposals, InvalidProposal());
        return _proposals[proposalId];
    }

    /// @inheritdoc ISlashingManager
    function isBanned(address node) external view returns (bool) {
        return banned[node];
    }

    // ======================
    // Admin Functions
    // ======================

    /// @inheritdoc ISlashingManager
    function setSlashPolicy(
        bytes32 reason,
        SlashPolicy calldata policy
    ) external onlyRole(GOVERNANCE_ROLE) {
        require(reason != bytes32(0), InvalidPolicy());
        require(policy.enabled, InvalidPolicy());
        require(
            policy.ticketPenalty > 0 || policy.licensePenalty > 0,
            InvalidPolicy()
        );

        if (policy.requiresProof) {
            // Attestation-based slashes: no proofVerifier needed, no appeal window
            require(policy.appealWindow == 0, InvalidPolicy());
        } else {
            require(policy.appealWindow > 0, InvalidPolicy());
        }

        slashPolicies[reason] = policy;
        emit SlashPolicyUpdated(reason, policy);
    }

    /// @inheritdoc ISlashingManager
    function setBondingRegistry(
        IBondingRegistry newBondingRegistry
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(address(newBondingRegistry) != address(0), ZeroAddress());
        bondingRegistry = newBondingRegistry;
    }

    /// @notice Updates the ciphernode registry contract
    /// @param newCiphernodeRegistry The new ICiphernodeRegistry contract
    function setCiphernodeRegistry(
        ICiphernodeRegistry newCiphernodeRegistry
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(address(newCiphernodeRegistry) != address(0), ZeroAddress());
        ciphernodeRegistry = newCiphernodeRegistry;
    }

    /// @notice Updates the Enclave contract
    /// @param newEnclave The new IEnclave contract
    function setEnclave(
        IEnclave newEnclave
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(address(newEnclave) != address(0), ZeroAddress());
        enclave = newEnclave;
    }

    /// @inheritdoc ISlashingManager
    function setE3RefundManager(
        IE3RefundManager newRefundManager
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(address(newRefundManager) != address(0), ZeroAddress());
        e3RefundManager = newRefundManager;
    }

    /// @inheritdoc ISlashingManager
    function addSlasher(address slasher) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(slasher != address(0), ZeroAddress());
        _grantRole(SLASHER_ROLE, slasher);
    }

    /// @inheritdoc ISlashingManager
    function removeSlasher(
        address slasher
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        _revokeRole(SLASHER_ROLE, slasher);
    }

    // ======================
    // Slashing Functions
    // ======================

    /// @inheritdoc ISlashingManager
    /// @dev Lane A: Permissionless committee attestation-based slash. Anyone can call.
    ///      Atomically proposes, verifies committee vote signatures, and executes slash.
    ///      Evidence format:
    ///      `abi.encode(uint256 proofType,
    ///         address[] voters, bool[] agrees, bytes32[] dataHashes, bytes[] signatures)`
    ///      Each voter must sign via `personal_sign`/`signMessage()` (EIP-191 prefixed):
    ///      `personal_sign(keccak256(abi.encode(VOTE_TYPEHASH,
    ///         block.chainid, e3Id, accusationId, voter, agrees, dataHash)))`
    ///      where `accusationId = keccak256(abi.encodePacked(block.chainid, e3Id, operator, proofType))`
    function proposeSlash(
        uint256 e3Id,
        address operator,
        bytes32 reason,
        bytes calldata proof
    ) external returns (uint256 proposalId) {
        require(operator != address(0), ZeroAddress());

        SlashPolicy memory policy = slashPolicies[reason];
        require(policy.enabled, SlashReasonDisabled());
        require(policy.requiresProof, InvalidPolicy());
        require(proof.length != 0, ProofRequired());
        require(
            ciphernodeRegistry.isCommitteeMember(e3Id, operator),
            OperatorNotInCommittee()
        );

        // Evidence replay protection — reason-independent to prevent cross-reason replay
        bytes32 evidenceKey = keccak256(
            abi.encode(e3Id, operator, keccak256(proof))
        );
        require(!evidenceConsumed[evidenceKey], DuplicateEvidence());
        evidenceConsumed[evidenceKey] = true;

        // Verify committee attestation: vote signatures and quorum
        _verifyAttestationEvidence(proof, e3Id, operator);

        // Create proposal
        proposalId = totalProposals;
        totalProposals = proposalId + 1;

        SlashProposal storage p = _proposals[proposalId];
        p.e3Id = e3Id;
        p.operator = operator;
        p.reason = reason;
        p.ticketAmount = policy.ticketPenalty;
        p.licenseAmount = policy.licensePenalty;
        p.proposedAt = block.timestamp;
        p.executableAt = block.timestamp;
        p.proposer = msg.sender;
        p.proofHash = keccak256(proof);
        p.proofVerified = true;
        p.banNode = policy.banNode;
        p.affectsCommittee = policy.affectsCommittee;
        p.failureReason = policy.failureReason;

        emit SlashProposed(
            proposalId,
            e3Id,
            operator,
            reason,
            policy.ticketPenalty,
            policy.licensePenalty,
            block.timestamp,
            msg.sender
        );

        _executeSlash(proposalId);
    }

    /// @inheritdoc ISlashingManager
    /// @dev Lane B: Evidence-based slash with appeal window. SLASHER_ROLE required.
    function proposeSlashEvidence(
        uint256 e3Id,
        address operator,
        bytes32 reason,
        bytes calldata evidence
    ) external onlySlasher returns (uint256 proposalId) {
        require(operator != address(0), ZeroAddress());

        SlashPolicy memory policy = slashPolicies[reason];
        require(policy.enabled, SlashReasonDisabled());
        require(!policy.requiresProof, InvalidPolicy());

        // Evidence replay protection — reason-independent to prevent cross-reason replay
        bytes32 evidenceKey = keccak256(
            abi.encode(e3Id, operator, keccak256(evidence))
        );
        require(!evidenceConsumed[evidenceKey], DuplicateEvidence());
        evidenceConsumed[evidenceKey] = true;

        proposalId = totalProposals;
        totalProposals = proposalId + 1;

        uint256 executableAt = block.timestamp + policy.appealWindow;
        SlashProposal storage p = _proposals[proposalId];
        p.e3Id = e3Id;
        p.operator = operator;
        p.reason = reason;
        p.ticketAmount = policy.ticketPenalty;
        p.licenseAmount = policy.licensePenalty;
        p.proposedAt = block.timestamp;
        p.executableAt = executableAt;
        p.proposer = msg.sender;
        p.proofHash = keccak256(evidence);
        // Snapshot behavioral flags from policy at proposal time
        // to prevent execution drift if policy is modified during appeal window
        p.banNode = policy.banNode;
        p.affectsCommittee = policy.affectsCommittee;
        p.failureReason = policy.failureReason;

        emit SlashProposed(
            proposalId,
            e3Id,
            operator,
            reason,
            policy.ticketPenalty,
            policy.licensePenalty,
            executableAt,
            msg.sender
        );
    }

    /// @inheritdoc ISlashingManager
    /// @dev Only for evidence-based slashes (Lane B). Proof-based slashes execute atomically.
    function executeSlash(uint256 proposalId) external {
        require(proposalId < totalProposals, InvalidProposal());
        SlashProposal storage p = _proposals[proposalId];
        require(!p.executed, AlreadyExecuted());

        // Use snapshotted requiresProof state: proof-based slashes are already executed atomically in proposeSlash
        require(!p.proofVerified, InvalidPolicy());

        // Evidence mode: check appeal window
        require(block.timestamp >= p.executableAt, AppealWindowActive());
        if (p.appealed) {
            require(p.resolved, AppealPending());
            require(!p.appealUpheld, AppealUpheld());
        }

        _executeSlash(proposalId);
    }

    // ======================
    // Internal Execution
    // ======================

    /// @dev Verifies committee attestation evidence for Lane A slashes.
    ///      Decodes the attestation, checks quorum (>= threshold M), verifies each voter's
    ///      ECDSA signature against the VOTE_TYPEHASH-structured digest, and confirms each
    ///      voter is a committee member. Voters must be sorted ascending to prevent duplicates.
    function _verifyAttestationEvidence(
        bytes calldata proof,
        uint256 e3Id,
        address operator
    ) internal view {
        (
            uint256 proofType,
            address[] memory voters,
            bool[] memory agrees,
            bytes32[] memory dataHashes,
            bytes[] memory signatures
        ) = abi.decode(proof, (uint256, address[], bool[], bytes32[], bytes[]));

        uint256 numVotes = voters.length;
        require(
            numVotes == agrees.length &&
                numVotes == dataHashes.length &&
                numVotes == signatures.length,
            InvalidProof()
        );

        // Compute accusation ID matching AccusationManager::accusation_id() on the Rust side
        bytes32 accusationId = keccak256(
            abi.encodePacked(block.chainid, e3Id, operator, proofType)
        );

        // Get committee threshold — need at least M agreeing votes
        {
            (, uint32 thresholdM, , ) = ciphernodeRegistry
                .getCommitteeViability(e3Id);
            require(thresholdM > 0, InvalidProposal());
            require(numVotes >= thresholdM, InsufficientAttestations());
        }

        // Verify each vote signature and membership
        address prevVoter = address(0);
        for (uint256 i = 0; i < numVotes; i++) {
            address voter = voters[i];

            // Sorted ascending order prevents duplicate voters
            require(voter > prevVoter, DuplicateVoter());
            prevVoter = voter;

            // The accused cannot vote on their own accusation (conflict of interest)
            require(voter != operator, VoterIsAccused());

            // All votes must agree the proof is bad (fault confirmed)
            require(agrees[i], InvalidProof());

            // Verify voter is an active committee member for this E3
            require(
                ciphernodeRegistry.isCommitteeMemberActive(e3Id, voter),
                VoterNotInCommittee()
            );

            // Reconstruct vote digest and verify signature in a scoped block
            // to avoid stack-too-deep
            {
                bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
                    keccak256(
                        abi.encode(
                            VOTE_TYPEHASH,
                            block.chainid,
                            e3Id,
                            accusationId,
                            voter,
                            agrees[i],
                            dataHashes[i]
                        )
                    )
                );
                require(
                    ECDSA.recover(ethSignedHash, signatures[i]) == voter,
                    InvalidVoteSignature()
                );
            }
        }
    }

    /// @dev Executes a slash: applies financial penalties, optional ban, and committee expulsion.
    ///      Lane B: if the operator deregistered or exited during the appeal window, penalties
    ///      gracefully become 0 (BondingRegistry uses min(requested, available)). Accepted tradeoff.
    function _executeSlash(uint256 proposalId) internal {
        SlashProposal storage p = _proposals[proposalId];
        p.executed = true;

        uint256 actualTicketSlashed = 0;

        // Execute financial penalties
        if (p.ticketAmount > 0) {
            actualTicketSlashed = bondingRegistry.slashTicketBalance(
                p.operator,
                p.ticketAmount,
                p.reason
            );
        }

        if (p.licenseAmount > 0) {
            bondingRegistry.slashLicenseBond(
                p.operator,
                p.licenseAmount,
                p.reason
            );
        }

        // Ban node if snapshotted policy requires it
        if (p.banNode) {
            banned[p.operator] = true;
            emit NodeBanUpdated(p.operator, true, p.reason, address(this));
        }

        // Committee expulsion for E3-scoped slashes (uses snapshotted behavioral flags)
        // expelCommitteeMember returns (activeCount, thresholdM) — one call instead of three
        if (p.affectsCommittee) {
            (uint256 activeCount, uint32 thresholdM) = ciphernodeRegistry
                .expelCommitteeMember(p.e3Id, p.operator, p.reason);

            // If active count drops below M, fail the E3
            if (activeCount < thresholdM && p.failureReason > 0) {
                // NOTE: catch block must not be empty (solc optimizer bug, see below)
                // solhint-disable-next-line no-empty-blocks
                try enclave.onE3Failed(p.e3Id, p.failureReason) {
                    // Side effects occur in the external call
                } catch {
                    // E3 already failed or other error — slash still proceeds
                    emit RoutingFailed(p.e3Id, 0);
                }
            }
        }

        // Escrow slashed ticket funds for deferred distribution.
        // Self-call for try/catch atomicity — on failure, funds stay in BondingRegistry.
        if (actualTicketSlashed > 0) {
            // NOTE: catch must not be empty — solc >=0.8.28 optimizer bug.
            // solhint-disable-next-line no-empty-blocks
            try
                this.escrowSlashedFundsToRefund(p.e3Id, actualTicketSlashed)
            {} catch {
                emit RoutingFailed(p.e3Id, actualTicketSlashed);
            }
        }

        emit SlashExecuted(
            proposalId,
            p.e3Id,
            p.operator,
            p.reason,
            p.ticketAmount,
            p.licenseAmount,
            true
        );
    }

    /// @inheritdoc ISlashingManager
    /// @dev Atomically redirects slashed funds to E3RefundManager escrow.
    ///      External with self-only access for try/catch atomicity.
    function escrowSlashedFundsToRefund(uint256 e3Id, uint256 amount) external {
        require(msg.sender == address(this), Unauthorized());
        address refundManager = address(e3RefundManager);
        require(refundManager != address(0), ZeroAddress());
        bondingRegistry.redirectSlashedTicketFunds(refundManager, amount);
        enclave.escrowSlashedFunds(e3Id, amount);
        emit SlashedFundsEscrowedToRefund(e3Id, amount);
    }

    // ======================
    // Appeal Functions
    // ======================

    /// @inheritdoc ISlashingManager
    /// @dev Only the accused operator may appeal (no delegate support). Consider an `appealDelegate`
    ///      mapping for production to handle lost-key or banned-operator scenarios.
    function fileAppeal(uint256 proposalId, string calldata evidence) external {
        require(proposalId < totalProposals, InvalidProposal());
        SlashProposal storage p = _proposals[proposalId];

        // Only the accused can appeal
        require(msg.sender == p.operator, Unauthorized());
        // Only within the appeal window
        require(block.timestamp < p.executableAt, AppealWindowExpired());
        // Only once
        require(!p.appealed, AlreadyAppealed());
        // Cannot appeal proof-verified slashes (they have no appeal window)
        require(!p.proofVerified, InvalidProposal());

        p.appealed = true;

        emit AppealFiled(proposalId, p.operator, p.reason, evidence);
    }

    /// @inheritdoc ISlashingManager
    function resolveAppeal(
        uint256 proposalId,
        bool appealUpheld,
        string calldata resolution
    ) external onlyGovernance {
        require(proposalId < totalProposals, InvalidProposal());
        SlashProposal storage p = _proposals[proposalId];

        require(p.appealed, InvalidProposal());
        require(!p.resolved, AlreadyResolved());

        p.resolved = true;
        p.appealUpheld = appealUpheld;

        emit AppealResolved(
            proposalId,
            p.operator,
            appealUpheld,
            msg.sender,
            resolution
        );
    }

    // ======================
    // Ban Management
    // ======================

    /// @inheritdoc ISlashingManager
    function updateBanStatus(
        address node,
        bool status,
        bytes32 reason
    ) external onlyGovernance {
        require(node != address(0), ZeroAddress());

        banned[node] = status;
        emit NodeBanUpdated(node, status, reason, msg.sender);
    }
}
