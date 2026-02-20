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
import { ICircuitVerifier } from "../interfaces/ICircuitVerifier.sol";

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
     * @param _bondingRegistry Address of the bonding registry contract
     * @param _ciphernodeRegistry Address of the ciphernode registry contract
     * @param _enclave Address of the Enclave contract
     */
    constructor(
        address admin,
        address _bondingRegistry,
        address _ciphernodeRegistry,
        address _enclave
    ) {
        require(admin != address(0), ZeroAddress());
        require(_bondingRegistry != address(0), ZeroAddress());
        require(_ciphernodeRegistry != address(0), ZeroAddress());
        require(_enclave != address(0), ZeroAddress());

        bondingRegistry = IBondingRegistry(_bondingRegistry);
        ciphernodeRegistry = ICiphernodeRegistry(_ciphernodeRegistry);
        enclave = IEnclave(_enclave);

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
            require(policy.proofVerifier != address(0), VerifierNotSet());
            require(policy.appealWindow == 0, InvalidPolicy());
        } else {
            require(policy.appealWindow > 0, InvalidPolicy());
        }

        slashPolicies[reason] = policy;
        emit SlashPolicyUpdated(reason, policy);
    }

    /// @inheritdoc ISlashingManager
    function setBondingRegistry(
        address newBondingRegistry
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(newBondingRegistry != address(0), ZeroAddress());
        bondingRegistry = IBondingRegistry(newBondingRegistry);
    }

    /// @notice Updates the ciphernode registry contract address
    /// @param newCiphernodeRegistry Address of the new ICiphernodeRegistry contract
    function setCiphernodeRegistry(
        address newCiphernodeRegistry
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(newCiphernodeRegistry != address(0), ZeroAddress());
        ciphernodeRegistry = ICiphernodeRegistry(newCiphernodeRegistry);
    }

    /// @notice Updates the Enclave contract address
    /// @param newEnclave Address of the new IEnclave contract
    function setEnclave(
        address newEnclave
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(newEnclave != address(0), ZeroAddress());
        enclave = IEnclave(newEnclave);
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
    /// @dev Lane A: Permissionless proof-based slash. Anyone can call.
    ///      Atomically proposes, verifies operator signature + ZK proof, and executes slash.
    ///      Evidence format:
    ///      `abi.encode(bytes zkProof, bytes32[] publicInputs,
    ///         bytes signature,
    ///         uint256 chainId,
    ///         uint256 proofType,
    ///         address verifier)`
    ///      The operator must have signed:
    ///      `keccak256(abi.encode(PROOF_PAYLOAD_TYPEHASH,
    ///         chainId,
    ///         e3Id,
    ///         proofType,
    ///         keccak256(zkProof),
    ///         keccak256(abi.encodePacked(publicInputs))))`
    ///      This prevents:
    ///        - Arbitrary proof submission (attacker can't forge operator's signature)
    ///        - Cross-E3 replay (e3Id is in the signed message)`
    ///        - Cross-chain replay (chainId is in the signed message)`
    ///        - Verifier-upgrade attacks (verifier in evidence must match policy's current verifier)`
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

        // Evidence replay protection — reason-independent to prevent cross-reason replay (M-05)
        bytes32 evidenceKey = keccak256(
            abi.encode(e3Id, operator, keccak256(proof))
        );
        require(!evidenceConsumed[evidenceKey], DuplicateEvidence());
        evidenceConsumed[evidenceKey] = true;

        // Verify evidence: signature, committee membership, and ZK proof
        _verifyProofEvidence(proof, e3Id, operator, policy.proofVerifier);

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

        _executeSlash(proposalId, policy);
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

        // Evidence replay protection — reason-independent to prevent cross-reason replay (M-05)
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

        SlashPolicy memory policy = slashPolicies[p.reason];

        // Proof-based slashes are already executed atomically in proposeSlash
        require(!policy.requiresProof, InvalidPolicy());

        // Evidence mode: check appeal window
        require(block.timestamp >= p.executableAt, AppealWindowActive());
        if (p.appealed) {
            require(p.resolved, AppealPending());
            require(!p.appealUpheld, AppealUpheld());
        }

        _executeSlash(proposalId, policy);
    }

    // ======================
    // Internal Execution
    // ======================

    /// @dev Verifies the operator is/was a committee member for the given E3.
    function _verifyCommitteeMembership(
        uint256 e3Id,
        address operator
    ) internal view {
        address[] memory committeeNodes = ciphernodeRegistry.getCommitteeNodes(
            e3Id
        );
        bool isMember = false;
        for (uint256 i = 0; i < committeeNodes.length; i++) {
            if (committeeNodes[i] == operator) {
                isMember = true;
                break;
            }
        }
        require(isMember, OperatorNotInCommittee());
    }

    /// @dev Decodes evidence, verifies operator signature, committee membership,
    ///      and that the ZK proof is invalid (fault confirmed).
    ///      Evidence format:
    ///      `abi.encode(bytes zkProof, bytes32[] publicInputs,
    ///         bytes signature,
    ///         uint256 chainId,
    ///         uint256 proofType,
    ///         address verifier)`
    function _verifyProofEvidence(
        bytes calldata proof,
        uint256 e3Id,
        address operator,
        address policyVerifier
    ) internal view {
        (
            bytes memory zkProof,
            bytes32[] memory publicInputs,
            bytes memory signature,
            uint256 chainId,
            uint256 proofType,
            address signedVerifier
        ) = abi.decode(
                proof,
                (bytes, bytes32[], bytes, uint256, uint256, address)
            );

        // 1. Verify verifier in evidence matches policy's current verifier.
        require(signedVerifier == policyVerifier, VerifierMismatch());

        // 2. Verify the operator signed this exact proof payload.
        bytes32 messageHash = keccak256(
            abi.encode(
                PROOF_PAYLOAD_TYPEHASH,
                chainId,
                e3Id,
                proofType,
                keccak256(zkProof),
                keccak256(abi.encodePacked(publicInputs))
            )
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            messageHash
        );
        address recoveredSigner = ECDSA.recover(ethSignedHash, signature);
        require(recoveredSigner == operator, SignerIsNotOperator());

        // 3. Verify committee membership.
        _verifyCommitteeMembership(e3Id, operator);

        // 4. Re-verify the ZK proof on-chain (INVERTED: must FAIL to confirm fault).
        //    The staticcall MUST succeed — if the verifier reverts or doesn't exist,
        //    we cannot determine fault and must not slash (M-04 fix).
        (bool callSuccess, bytes memory returnData) = policyVerifier.staticcall(
            abi.encodeCall(ICircuitVerifier.verify, (zkProof, publicInputs))
        );
        require(callSuccess, VerifierCallFailed());
        bool proofValid = abi.decode(returnData, (bool));
        if (proofValid) revert ProofIsValid();
    }

    /**
     * @notice Internal function that executes a slash and handles committee expulsion
     * @dev For Lane B (delayed execution), the operator may have deregistered during the appeal
     *      window. BondingRegistry.slashTicketBalance and slashLicenseBond use Math.min(requested,
     *      available), so zero-balance operators receive a zero slash gracefully. The exit queue's
     *      slashPendingAssets(includeLockedAssets=true) covers operators mid-exit. If the operator
     *      has already claimed their exit, funds are gone and the slash amount becomes 0. This is
     *      an accepted tradeoff for the appeal window design.
     * @param proposalId ID of the proposal to execute
     * @param policy The slash policy for this proposal
     */
    function _executeSlash(
        uint256 proposalId,
        SlashPolicy memory policy
    ) internal {
        SlashProposal storage p = _proposals[proposalId];
        p.executed = true;

        // Execute financial penalties
        if (p.ticketAmount > 0) {
            bondingRegistry.slashTicketBalance(
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

        // Ban node if policy requires it
        if (policy.banNode) {
            banned[p.operator] = true;
            emit NodeBanUpdated(p.operator, true, p.reason, address(this));
        }

        // Committee expulsion for E3-scoped slashes
        // expelCommitteeMember returns (activeCount, thresholdM) — one call instead of three
        if (policy.affectsCommittee) {
            (uint256 activeCount, uint32 thresholdM) = ciphernodeRegistry
                .expelCommitteeMember(p.e3Id, p.operator, p.reason);

            // If active count drops below M, fail the E3
            if (activeCount < thresholdM && policy.failureReason > 0) {
                enclave.onE3Failed(p.e3Id, policy.failureReason);
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

    // ======================
    // Appeal Functions
    // ======================

    /// @inheritdoc ISlashingManager
    /// @dev Only the accused operator can file an appeal. No delegate, multi-sig, or representative
    ///      patterns exist. If the operator has lost access to their key or been banned, they cannot
    ///      appeal. Consider adding an appealDelegate mapping for production to allow a designated
    ///      representative to appeal on behalf of the operator.
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
