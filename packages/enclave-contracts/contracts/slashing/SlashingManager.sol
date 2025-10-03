// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import {
    AccessControl
} from "@openzeppelin/contracts/access/AccessControl.sol";
import { ISlashingManager } from "../interfaces/ISlashingManager.sol";
import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";
import { ISlashVerifier } from "../interfaces/ISlashVerifier.sol";

/**
 * @title SlashingManager
 * @notice Implementation of slashing management with proposal, appeal, and execution workflows
 * @dev Role-based access control for slashers, verifiers, and governance with configurable slash policies
 */
contract SlashingManager is ISlashingManager, AccessControl {
    // ======================
    // Constants & Roles
    // ======================

    /// @notice Role identifier for accounts authorized to propose and execute slashes
    bytes32 public constant SLASHER_ROLE = keccak256("SLASHER_ROLE");

    /// @notice Role identifier for accounts authorized to verify cryptographic proofs in slash proposals
    bytes32 public constant VERIFIER_ROLE = keccak256("VERIFIER_ROLE");

    /// @notice Role identifier for governance accounts that can configure policies, resolve appeals, and manage bans
    bytes32 public constant GOVERNANCE_ROLE = keccak256("GOVERNANCE_ROLE");

    // ======================
    // Storage
    // ======================

    /// @notice Reference to the bonding registry contract where slash penalties are executed
    /// @dev Used to call slashTicketBalance() and slashLicenseBond() when executing slashes
    IBondingRegistry public bondingRegistry;

    /// @notice Mapping from slash reason hash to its configured policy
    /// @dev Stores penalty amounts, proof requirements, and appeal settings for each slash type
    mapping(bytes32 reason => SlashPolicy policy) public slashPolicies;

    /// @notice Internal storage for all slash proposals indexed by proposal ID
    /// @dev Sequentially indexed starting from 0, accessed via getSlashProposal()
    mapping(uint256 proposalId => SlashProposal proposal) internal _proposals;

    /// @notice Counter for total number of slash proposals ever created
    /// @dev Also serves as the next proposal ID to be assigned
    uint256 public totalProposals;

    /// @notice Mapping tracking which nodes are currently banned from the network
    /// @dev Set to true when a node is banned (either via executeSlash or banNode), false when unbanned
    mapping(address node => bool banned) public banned;

    // ======================
    // Modifiers
    // ======================

    /// @notice Restricts function access to accounts with SLASHER_ROLE
    /// @dev Reverts with Unauthorized() if caller lacks the role
    modifier onlySlasher() {
        if (!hasRole(SLASHER_ROLE, msg.sender)) revert Unauthorized();
        _;
    }

    /// @notice Restricts function access to accounts with VERIFIER_ROLE
    /// @dev Reverts with Unauthorized() if caller lacks the role
    modifier onlyVerifier() {
        if (!hasRole(VERIFIER_ROLE, msg.sender)) revert Unauthorized();
        _;
    }

    /// @notice Restricts function access to accounts with GOVERNANCE_ROLE
    /// @dev Reverts with Unauthorized() if caller lacks the role
    modifier onlyGovernance() {
        if (!hasRole(GOVERNANCE_ROLE, msg.sender)) revert Unauthorized();
        _;
    }

    // ======================
    // Constructor
    // ======================

    /**
     * @notice Initializes the SlashingManager contract with admin and bonding registry
     * @dev Sets up initial role assignments and bonding registry reference
     * @param admin Address to receive DEFAULT_ADMIN_ROLE and GOVERNANCE_ROLE
     * @param _bondingRegistry Address of the bonding registry contract for executing slashes
     * Requirements:
     * - admin must not be zero address
     * - _bondingRegistry must not be zero address
     */
    constructor(address admin, address _bondingRegistry) {
        require(admin != address(0), ZeroAddress());
        require(_bondingRegistry != address(0), ZeroAddress());

        bondingRegistry = IBondingRegistry(_bondingRegistry);

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
            // TODO: Should we allow appeal window for proof required?
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

    /// @inheritdoc ISlashingManager
    function addVerifier(
        address verifier
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(verifier != address(0), ZeroAddress());
        _grantRole(VERIFIER_ROLE, verifier);
    }

    /// @inheritdoc ISlashingManager
    function removeVerifier(
        address verifier
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        _revokeRole(VERIFIER_ROLE, verifier);
    }

    // ======================
    // Slashing Functions
    // ======================

    /// @inheritdoc ISlashingManager
    function proposeSlash(
        address operator,
        bytes32 reason,
        bytes calldata proof
    )
        external
        // TODO: Do we need an onlySlasher modifier?
        // Can anyone propose a slash?
        onlySlasher
        returns (uint256 proposalId)
    {
        require(operator != address(0), ZeroAddress());

        SlashPolicy memory policy = slashPolicies[reason];
        require(policy.enabled, SlashReasonDisabled());

        proposalId = totalProposals;
        uint256 executableAt = block.timestamp + policy.appealWindow;
        SlashProposal storage p = _proposals[proposalId];

        p.operator = operator;
        p.reason = reason;
        p.ticketAmount = policy.ticketPenalty;
        p.licenseAmount = policy.licensePenalty;
        p.proposedAt = block.timestamp;
        p.executableAt = executableAt;
        p.proposer = msg.sender;
        p.proofHash = keccak256(proof);

        if (policy.requiresProof) {
            require(proof.length != 0, ProofRequired());
            bool ok = ISlashVerifier(policy.proofVerifier).verify(
                proposalId,
                proof
            );
            require(ok, InvalidProof());
            p.proofVerified = true;
        }

        emit SlashProposed(
            proposalId,
            operator,
            reason,
            policy.ticketPenalty,
            policy.licensePenalty,
            executableAt,
            msg.sender
        );

        totalProposals = proposalId + 1;
    }

    /// @inheritdoc ISlashingManager
    function executeSlash(uint256 proposalId) external onlySlasher {
        require(proposalId < totalProposals, InvalidProposal());
        SlashProposal storage p = _proposals[proposalId];

        // Has already been executed?
        require(!p.executed, AlreadyExecuted());
        p.executed = true;

        SlashPolicy memory policy = slashPolicies[p.reason];

        if (policy.requiresProof) {
            // Appeal window is 0 by policy validation, so we dont check for appeal gating
            require(p.proofVerified, InvalidProof());
        } else {
            // Evidence mode with appeals
            require(block.timestamp >= p.executableAt, AppealWindowActive());
            if (p.appealed) {
                require(p.resolved, AppealPending());
                require(!p.appealUpheld, AppealUpheld()); // approved = appeal upheld => cancel slash, return?
            }
        }

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

        if (policy.banNode) {
            banned[p.operator] = true;
            emit NodeBanned(p.operator, p.reason, address(this));
        }

        emit SlashExecuted(
            proposalId,
            p.operator,
            p.reason,
            p.ticketAmount,
            p.licenseAmount,
            p.executed
        );
    }

    // ======================
    // Appeal Functions
    // ======================

    /// @inheritdoc ISlashingManager
    function fileAppeal(uint256 proposalId, string calldata evidence) external {
        require(proposalId < totalProposals, InvalidProposal());
        SlashProposal storage p = _proposals[proposalId];

        // Only the accused can appeal
        require(msg.sender == p.operator, Unauthorized());
        // Only in the window
        require(block.timestamp < p.executableAt, AppealWindowExpired());
        // Only once
        require(!p.appealed, AlreadyAppealed());

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
        p.appealUpheld = appealUpheld; // true => cancel slash, false => slash stands

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
    function banNode(address node, bytes32 reason) external onlyGovernance {
        require(node != address(0), ZeroAddress());

        banned[node] = true;
        emit NodeBanned(node, reason, msg.sender);
    }

    /// @inheritdoc ISlashingManager
    function unbanNode(address node) external onlyGovernance {
        require(node != address(0), ZeroAddress());

        banned[node] = false;
        emit NodeUnbanned(node, msg.sender);
    }
}
