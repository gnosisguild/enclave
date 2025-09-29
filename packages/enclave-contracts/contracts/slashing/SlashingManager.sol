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
 * @notice Manages slashing proposals, appeals, and execution for the bonding system
 * @dev UUPS upgradeable contract with role-based access control
 */
contract SlashingManager is ISlashingManager, AccessControl {
    // ======================
    // Constants & Roles
    // ======================

    bytes32 public constant SLASHER_ROLE = keccak256("SLASHER_ROLE");
    bytes32 public constant VERIFIER_ROLE = keccak256("VERIFIER_ROLE");
    bytes32 public constant GOVERNANCE_ROLE = keccak256("GOVERNANCE_ROLE");

    // ======================
    // Storage
    // ======================

    /// @notice Bonding registry contract
    IBondingRegistry public bondingRegistry;

    /// @notice Slash policies by reason hash
    mapping(bytes32 reason => SlashPolicy policy) public slashPolicies;

    /// @notice All slash proposals
    mapping(uint256 proposalId => SlashProposal proposal) public proposals;

    /// @notice Total number of proposals created
    uint256 public totalProposals;

    /// @notice Banned nodes
    mapping(address node => bool banned) public banned;

    // ======================
    // Modifiers
    // ======================

    modifier onlySlasher() {
        if (!hasRole(SLASHER_ROLE, msg.sender)) revert Unauthorized();
        _;
    }

    modifier onlyVerifier() {
        if (!hasRole(VERIFIER_ROLE, msg.sender)) revert Unauthorized();
        _;
    }

    modifier onlyGovernance() {
        if (!hasRole(GOVERNANCE_ROLE, msg.sender)) revert Unauthorized();
        _;
    }

    modifier notBanned(address node) {
        if (banned[node]) revert CiphernodeBanned();
        _;
    }

    // ======================
    // Constructor
    // ======================

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

    function getSlashPolicy(
        bytes32 reason
    ) external view returns (SlashPolicy memory) {
        return slashPolicies[reason];
    }

    function getSlashProposal(
        uint256 proposalId
    ) external view returns (SlashProposal memory) {
        require(proposalId < totalProposals, InvalidProposal());
        return proposals[proposalId];
    }

    function isBanned(address node) external view returns (bool) {
        return banned[node];
    }

    // ======================
    // Admin Functions
    // ======================

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

    function setBondingRegistry(
        address newBondingRegistry
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(newBondingRegistry != address(0), ZeroAddress());
        bondingRegistry = IBondingRegistry(newBondingRegistry);
    }

    function addSlasher(address slasher) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(slasher != address(0), ZeroAddress());
        _grantRole(SLASHER_ROLE, slasher);
    }

    function removeSlasher(
        address slasher
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        _revokeRole(SLASHER_ROLE, slasher);
    }

    function addVerifier(
        address verifier
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(verifier != address(0), ZeroAddress());
        _grantRole(VERIFIER_ROLE, verifier);
    }

    function removeVerifier(
        address verifier
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        _revokeRole(VERIFIER_ROLE, verifier);
    }

    // ======================
    // Slashing Functions
    // ======================

    function proposeSlash(
        address operator,
        bytes32 reason,
        bytes calldata proof
    ) external onlySlasher notBanned(operator) returns (uint256 proposalId) {
        require(operator != address(0), ZeroAddress());

        SlashPolicy storage policy = slashPolicies[reason];
        require(policy.enabled, SlashReasonDisabled());

        uint256 nextId = totalProposals;
        bool proofVerified = false;

        if (policy.requiresProof) {
            require(proof.length != 0, ProofRequired());
            proofVerified = ISlashVerifier(policy.proofVerifier).verify(
                nextId,
                proof
            );
            require(proofVerified, InvalidProof());
        }

        uint256 executableAt = block.timestamp + policy.appealWindow;

        proposals[nextId] = SlashProposal({
            operator: operator,
            reason: reason,
            ticketAmount: policy.ticketPenalty,
            licenseAmount: policy.licensePenalty,
            executedTicket: false,
            executedLicense: false,
            appealed: false,
            resolved: false,
            approved: false,
            proposedAt: block.timestamp,
            executableAt: executableAt,
            proposer: msg.sender,
            proofHash: keccak256(proof),
            proofVerified: proofVerified
        });

        emit SlashProposed(
            nextId,
            operator,
            reason,
            policy.ticketPenalty,
            policy.licensePenalty,
            executableAt,
            msg.sender
        );

        totalProposals = nextId + 1;
        return nextId;
    }

    function executeSlash(uint256 proposalId) external onlySlasher {
        require(proposalId < totalProposals, InvalidProposal());
        SlashProposal storage p = proposals[proposalId];

        // Has already been executed?
        require(!(p.executedTicket && p.executedLicense), AlreadyExecuted());

        SlashPolicy storage policy = slashPolicies[p.reason];

        if (policy.requiresProof) {
            // Appeal window is 0 by policy validation, so we dont check for appeal gating
            require(p.proofVerified, InvalidProof());
        } else {
            // Evidence mode with appeals
            require(block.timestamp >= p.executableAt, AppealWindowActive());
            if (p.appealed) {
                require(p.resolved, AppealPending());
                require(!p.approved, AppealUpheld()); // approved = appeal upheld => cancel slash, return?
            }
        }

        bool ticketExecuted = p.executedTicket;
        bool licenseExecuted = p.executedLicense;

        if (!p.executedTicket && p.ticketAmount > 0) {
            bondingRegistry.slashTicketBalance(
                p.operator,
                p.ticketAmount,
                p.reason
            );
            p.executedTicket = true;
            ticketExecuted = true;
        }

        if (!p.executedLicense && p.licenseAmount > 0) {
            bondingRegistry.slashLicenseBond(
                p.operator,
                p.licenseAmount,
                p.reason
            );
            p.executedLicense = true;
            licenseExecuted = true;
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
            ticketExecuted,
            licenseExecuted
        );
    }

    // ======================
    // Appeal Functions
    // ======================

    function fileAppeal(uint256 proposalId, string calldata evidence) external {
        require(proposalId < totalProposals, InvalidProposal());
        SlashProposal storage p = proposals[proposalId];

        // Only the accused can appeal
        require(msg.sender == p.operator, Unauthorized());
        // Only in the window
        require(block.timestamp < p.executableAt, AppealWindowExpired());
        // Only once
        require(!p.appealed, AlreadyAppealed());

        p.appealed = true;

        emit AppealFiled(proposalId, p.operator, p.reason, evidence);
    }

    function resolveAppeal(
        uint256 proposalId,
        bool approved,
        string calldata resolution
    ) external onlyGovernance {
        require(proposalId < totalProposals, InvalidProposal());
        SlashProposal storage p = proposals[proposalId];

        require(p.appealed, InvalidProposal());
        require(!p.resolved, AlreadyResolved());

        p.resolved = true;
        p.approved = approved; // true => cancel slash, false => slash stands

        emit AppealResolved(
            proposalId,
            p.operator,
            approved,
            msg.sender,
            resolution
        );
    }

    // ======================
    // Ban Management
    // ======================

    function banNode(address node, bytes32 reason) external onlyGovernance {
        require(node != address(0), ZeroAddress());

        banned[node] = true;
        emit NodeBanned(node, reason, msg.sender);
    }

    function unbanNode(address node) external onlyGovernance {
        require(node != address(0), ZeroAddress());

        banned[node] = false;
        emit NodeUnbanned(node, msg.sender);
    }
}
