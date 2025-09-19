// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import {
    UUPSUpgradeable
} from "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import {
    Initializable
} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {
    AccessControlUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import {
    PausableUpgradeable
} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import {
    ReentrancyGuardUpgradeable
} from "@openzeppelin/contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";

import { ISlashingManager } from "../interfaces/ISlashingManager.sol";
import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";
import { ISlashVerifier } from "../interfaces/ISlashVerifier.sol";

/**
 * @title SlashingManager
 * @notice Manages slashing proposals, appeals, and execution for the bonding system
 * @dev UUPS upgradeable contract with role-based access control
 */
contract SlashingManager is
    Initializable,
    UUPSUpgradeable,
    AccessControlUpgradeable,
    PausableUpgradeable,
    ReentrancyGuardUpgradeable,
    ISlashingManager
{
    // ======================
    // Constants & Roles
    // ======================

    bytes32 public constant SLASHER_ROLE = keccak256("SLASHER_ROLE");
    bytes32 public constant VERIFIER_ROLE = keccak256("VERIFIER_ROLE");
    bytes32 public constant GOVERNANCE_ROLE = keccak256("GOVERNANCE_ROLE");

    uint256 private constant BPS_DENOMINATOR = 10_000;

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
    // Storage Gaps for Upgrades
    // ======================

    uint256[50] private __gap;

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
    // Initialization
    // ======================

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    /**
     * @notice Initialize the contract
     * @param admin Contract admin (gets DEFAULT_ADMIN_ROLE and GOVERNANCE_ROLE)
     * @param _bondingRegistry Bonding registry contract address
     */
    function initialize(
        address admin,
        address _bondingRegistry
    ) external initializer {
        __AccessControl_init();
        __Pausable_init();
        __ReentrancyGuard_init();
        __UUPSUpgradeable_init();

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
        if (proposalId >= totalProposals) revert InvalidProposal();
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
    ) external onlyRole(DEFAULT_ADMIN_ROLE) {
        require(reason != bytes32(0), InvalidPolicy());
        if (policy.useTicketBps && policy.ticketPenalty > BPS_DENOMINATOR)
            revert InvalidPolicy();
        if (policy.useLicenseBps && policy.licensePenalty > BPS_DENOMINATOR)
            revert InvalidPolicy();

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
    )
        external
        onlySlasher
        whenNotPaused
        notBanned(operator)
        returns (uint256 proposalId)
    {
        SlashPolicy storage policy = slashPolicies[reason];
        require(policy.enabled, SlashReasonDisabled());
        require(!(policy.requiresProof && proof.length == 0), ProofRequired());

        uint256 ticketAmount = 0;
        uint256 licenseAmount = 0;

        if (policy.ticketPenalty > 0) {
            if (policy.useTicketBps) {
                uint256 ticketBalance = bondingRegistry.getTicketBalance(
                    operator
                );
                ticketAmount =
                    (ticketBalance * policy.ticketPenalty) /
                    BPS_DENOMINATOR;
            } else {
                ticketAmount = policy.ticketPenalty;
            }
        }

        if (policy.licensePenalty > 0) {
            if (policy.useLicenseBps) {
                uint256 bond = bondingRegistry.getLicenseBond(operator);
                licenseAmount =
                    (bond * policy.licensePenalty) /
                    BPS_DENOMINATOR;
            } else {
                licenseAmount = policy.licensePenalty;
            }
        }

        proposalId = totalProposals++;
        uint256 executableAt = block.timestamp + policy.appealWindow;

        bool proofVerified = false;
        if (policy.requiresProof) {
            proofVerified = ISlashVerifier(policy.proofVerifier).verify(
                proposalId,
                proof
            );
        }

        proposals[proposalId] = SlashProposal({
            operator: operator,
            reason: reason,
            ticketAmount: ticketAmount,
            licenseAmount: licenseAmount,
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
            proposalId,
            operator,
            reason,
            ticketAmount,
            licenseAmount,
            executableAt,
            msg.sender
        );
    }

    function executeSlash(
        uint256 proposalId
    ) external onlySlasher whenNotPaused nonReentrant {
        require(proposalId < totalProposals, InvalidProposal());

        SlashProposal storage proposal = proposals[proposalId];

        require(
            !(proposal.appealed && !proposal.resolved),
            AppealWindowActive()
        );
        require(!(proposal.resolved && proposal.approved), AlreadyExecuted());
        require(block.timestamp >= proposal.executableAt, AppealWindowActive());
        require(
            !(proposal.executedTicket && proposal.executedLicense),
            AlreadyExecuted()
        );

        bool ticketExecuted = proposal.executedTicket;
        bool licenseExecuted = proposal.executedLicense;

        // Ticket Slash
        if (!proposal.executedTicket && proposal.ticketAmount > 0) {
            bondingRegistry.slashTicketBalance(
                proposal.operator,
                proposal.ticketAmount,
                proposal.reason
            );
            proposal.executedTicket = true;
            ticketExecuted = true;
        }

        // License bond slash
        if (!proposal.executedLicense && proposal.licenseAmount > 0) {
            bondingRegistry.slashLicenseBond(
                proposal.operator,
                proposal.licenseAmount,
                proposal.reason
            );
            proposal.executedLicense = true;
            licenseExecuted = true;
        }

        SlashPolicy storage policy = slashPolicies[proposal.reason];
        if (policy.banNode) {
            banned[proposal.operator] = true;
            emit NodeBanned(proposal.operator, proposal.reason, address(this));
        }

        emit SlashExecuted(
            proposalId,
            proposal.operator,
            proposal.reason,
            proposal.ticketAmount,
            proposal.licenseAmount,
            ticketExecuted,
            licenseExecuted
        );
    }

    // ======================
    // Appeal Functions
    // ======================

    function fileAppeal(
        uint256 proposalId,
        string calldata evidence
    ) external whenNotPaused {
        require(proposalId < totalProposals, InvalidProposal());

        SlashProposal storage proposal = proposals[proposalId];
        require(msg.sender == proposal.operator, Unauthorized());
        require(!proposal.appealed, AlreadyAppealed());
        require(block.timestamp < proposal.executableAt, AppealWindowExpired());

        proposal.appealed = true;

        emit AppealFiled(
            proposalId,
            proposal.operator,
            proposal.reason,
            evidence
        );
    }

    function resolveAppeal(
        uint256 proposalId,
        bool approved,
        string calldata resolution
    ) external onlyGovernance {
        require(proposalId < totalProposals, InvalidProposal());

        SlashProposal storage proposal = proposals[proposalId];
        require(proposal.appealed, InvalidProposal());
        require(!proposal.resolved, AlreadyResolved());

        proposal.resolved = true;
        proposal.approved = approved;

        emit AppealResolved(
            proposalId,
            proposal.operator,
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

    function pause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        _unpause();
    }

    // ======================
    // Internal Functions
    // ======================

    function _authorizeUpgrade(
        address newImplementation
    ) internal override onlyRole(DEFAULT_ADMIN_ROLE) {}
}
