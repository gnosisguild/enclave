// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";

/**
 * @title CommitteeSortition
 * @notice Simple on-chain verification of ticket-based sortition
 * @dev Validates ticket submissions and tracks committee members
 *
 * Flow:
 * 1. Nodes perform sortition off-chain
 * 2. Selected nodes submit their winning ticket via submitTicket()
 * 3. Contract validates ticket against snapshot balance
 * 4. Contract tracks top N nodes by score
 */
contract CommitteeSortition {
    // ======================
    // Errors
    // ======================

    error InvalidTicketNumber();
    error NodeNotEligible();
    error NodeAlreadySubmitted();
    error SubmissionWindowClosed();
    error SubmissionWindowNotClosed();
    error CommitteeNotInitialized();
    error CommitteeAlreadyFinalized();
    error OnlyCiphernodeRegistry();

    // ======================
    // Events
    // ======================

    event TicketSubmitted(
        uint256 indexed e3Id,
        address indexed node,
        uint256 ticketNumber,
        uint256 score,
        bool addedToCommittee
    );

    event CommitteeFinalized(uint256 indexed e3Id, address[] committee);

    // ======================
    // Structs
    // ======================

    /// @notice Represents a node's ticket submission
    struct TicketSubmission {
        address node;
        uint256 ticketNumber;
        uint256 score;
        bool exists;
    }

    /// @notice Sortition state for an E3
    struct SortitionState {
        uint256 threshold; // Number of nodes needed
        uint256 seed; // Random seed for this E3
        uint256 requestBlock; // Block number when E3 was requested (for snapshot)
        uint256 submissionDeadline; // Timestamp when submission window closes
        bool finalized; // Whether committee has been finalized
        address[] topNodes; // Current top N nodes (sorted by score)
        mapping(address => TicketSubmission) submissions;
    }

    // ======================
    // Storage
    // ======================

    /// @notice Bonding registry for checking ticket balances
    IBondingRegistry public immutable bondingRegistry;

    /// @notice Ciphernode registry that can initialize sortitions
    address public immutable ciphernodeRegistry;

    /// @notice Default submission window duration (in seconds)
    uint256 public immutable submissionWindow;

    /// @notice Maps E3 ID to its sortition state
    mapping(uint256 => SortitionState) public sortitions;

    // ======================
    // Constructor
    // ======================

    constructor(
        address _bondingRegistry,
        address _ciphernodeRegistry,
        uint256 _submissionWindow
    ) {
        bondingRegistry = IBondingRegistry(_bondingRegistry);
        ciphernodeRegistry = _ciphernodeRegistry;
        submissionWindow = _submissionWindow;
    }

    // ======================
    // Main Functions
    // ======================

    /**
     * @notice Initialize sortition for an E3
     * @dev Only callable by ciphernode registry when committee is requested
     * @param e3Id The E3 identifier
     * @param threshold Number of committee members needed
     * @param seed Random seed for score computation
     * @param requestBlock Block number for snapshot validation
     */
    function initializeSortition(
        uint256 e3Id,
        uint256 threshold,
        uint256 seed,
        uint256 requestBlock
    ) external {
        require(msg.sender == ciphernodeRegistry, OnlyCiphernodeRegistry());
        SortitionState storage state = sortitions[e3Id];
        require(state.threshold == 0, CommitteeAlreadyFinalized());

        state.threshold = threshold;
        state.seed = seed;
        state.requestBlock = requestBlock;
        state.submissionDeadline = block.timestamp + submissionWindow;
        state.finalized = false;
    }

    /**
     * @notice Submit a ticket for sortition
     * @dev Nodes call this to submit their best ticket. Score is computed and verified on-chain.
     * @param e3Id The E3 identifier
     * @param ticketNumber The ticket number to submit (1 to available_tickets at snapshot)
     */
    function submitTicket(uint256 e3Id, uint256 ticketNumber) external {
        SortitionState storage state = sortitions[e3Id];

        // Check sortition is initialized
        require(state.threshold > 0, CommitteeNotInitialized());

        // Check submission window is still open
        require(
            block.timestamp <= state.submissionDeadline,
            SubmissionWindowClosed()
        );

        // Check not finalized
        require(!state.finalized, CommitteeAlreadyFinalized());

        // Check node hasn't already submitted
        if (state.submissions[msg.sender].exists) revert NodeAlreadySubmitted();

        // Check node is eligible (has ticket balance at snapshot)
        _validateNodeEligibility(msg.sender, ticketNumber, e3Id);

        // Compute score
        uint256 score = _computeTicketScore(
            msg.sender,
            ticketNumber,
            e3Id,
            state.seed
        );

        // Store submission
        state.submissions[msg.sender] = TicketSubmission({
            node: msg.sender,
            ticketNumber: ticketNumber,
            score: score,
            exists: true
        });

        // Try to insert into top N
        bool added = _tryInsertIntoTopN(state, msg.sender, score);

        emit TicketSubmitted(e3Id, msg.sender, ticketNumber, score, added);
    }

    /**
     * @notice Finalize the committee after submission window closes
     * @dev Can be called by anyone after the deadline. Sets finalized flag.
     * @param e3Id The E3 identifier
     * @return committee The final committee addresses
     */
    function finalizeCommittee(
        uint256 e3Id
    ) external returns (address[] memory committee) {
        SortitionState storage state = sortitions[e3Id];

        require(state.threshold > 0, CommitteeNotInitialized());
        require(!state.finalized, CommitteeAlreadyFinalized());
        require(
            block.timestamp > state.submissionDeadline,
            SubmissionWindowNotClosed()
        );

        state.finalized = true;
        committee = state.topNodes;

        emit CommitteeFinalized(e3Id, committee);
    }

    // ======================
    // View Functions
    // ======================

    /**
     * @notice Get the current top N nodes for an E3
     * @param e3Id The E3 identifier
     * @return Array of top N node addresses
     */
    function getTopNodes(
        uint256 e3Id
    ) external view returns (address[] memory) {
        return sortitions[e3Id].topNodes;
    }

    /**
     * @notice Get a node's submission for an E3
     * @param e3Id The E3 identifier
     * @param node The node address
     * @return The ticket submission
     */
    function getSubmission(
        uint256 e3Id,
        address node
    ) external view returns (TicketSubmission memory) {
        return sortitions[e3Id].submissions[node];
    }

    /**
     * @notice Compute the score for a ticket
     * @dev Public function to allow off-chain computation verification
     * @param node Node address
     * @param ticketNumber Ticket number (1 to N)
     * @param e3Id E3 identifier
     * @param seed Random seed
     * @return The computed score
     */
    function computeTicketScore(
        address node,
        uint256 ticketNumber,
        uint256 e3Id,
        uint256 seed
    ) external pure returns (uint256) {
        return _computeTicketScore(node, ticketNumber, e3Id, seed);
    }

    /**
     * @notice Get sortition information for an E3
     * @param e3Id The E3 identifier
     * @return threshold Number of committee members needed
     * @return seed Random seed
     * @return requestBlock Block number when E3 was requested
     * @return submissionDeadline Timestamp when submission window closes
     * @return finalized Whether committee has been finalized
     */
    function getSortitionInfo(
        uint256 e3Id
    )
        external
        view
        returns (
            uint256 threshold,
            uint256 seed,
            uint256 requestBlock,
            uint256 submissionDeadline,
            bool finalized
        )
    {
        SortitionState storage state = sortitions[e3Id];
        return (
            state.threshold,
            state.seed,
            state.requestBlock,
            state.submissionDeadline,
            state.finalized
        );
    }

    // ======================
    // Internal Functions
    // ======================

    /**
     * @notice Computes score = keccak256(node || ticketNumber || e3Id || seed)
     */
    function _computeTicketScore(
        address node,
        uint256 ticketNumber,
        uint256 e3Id,
        uint256 seed
    ) internal pure returns (uint256) {
        bytes32 hash = keccak256(
            abi.encodePacked(node, ticketNumber, e3Id, seed)
        );
        return uint256(hash);
    }

    /**
     * @notice Validates that a node is eligible to participate
     * @dev Uses snapshot of ticket balance at E3 request block for deterministic validation
     */
    function _validateNodeEligibility(
        address node,
        uint256 ticketNumber,
        uint256 e3Id
    ) internal view {
        if (ticketNumber == 0) revert InvalidTicketNumber();

        SortitionState storage state = sortitions[e3Id];

        // Get ticket balance at the time E3 was requested (snapshot)
        uint256 ticketBalance = bondingRegistry.getTicketBalanceAtBlock(
            node,
            state.requestBlock
        );
        uint256 ticketPrice = bondingRegistry.ticketPrice();

        if (ticketPrice == 0) revert InvalidTicketNumber();

        // Calculate available tickets at snapshot
        uint256 availableTickets = ticketBalance / ticketPrice;

        // Check ticket number is valid
        if (ticketNumber > availableTickets) revert InvalidTicketNumber();

        // Check node is eligible (has tickets at snapshot)
        if (availableTickets == 0) revert NodeNotEligible();
    }

    /**
     * @notice Try to insert node into top N sorted list
     * @dev Maintains sorted order by score (lowest first)
     * @return Whether node was added to top N
     */
    function _tryInsertIntoTopN(
        SortitionState storage state,
        address node,
        uint256 score
    ) internal returns (bool) {
        address[] storage topNodes = state.topNodes;

        // If list not full, insert in sorted order
        if (topNodes.length < state.threshold) {
            _insertSorted(state, node, score);
            return true;
        }

        // If list is full, only add if score is better than worst
        uint256 worstScore = state
            .submissions[topNodes[topNodes.length - 1]]
            .score;
        if (score < worstScore) {
            topNodes.pop(); // Remove worst
            _insertSorted(state, node, score);
            return true;
        }

        return false;
    }

    /**
     * @notice Insert node into sorted position (ascending by score)
     */
    function _insertSorted(
        SortitionState storage state,
        address node,
        uint256 score
    ) internal {
        address[] storage topNodes = state.topNodes;

        // Find insertion position
        uint256 insertPos = topNodes.length;
        for (uint256 i = 0; i < topNodes.length; i++) {
            uint256 existingScore = state.submissions[topNodes[i]].score;
            if (score < existingScore) {
                insertPos = i;
                break;
            }
        }

        // Insert at position
        topNodes.push(address(0)); // Extend array
        for (uint256 i = topNodes.length - 1; i > insertPos; i--) {
            topNodes[i] = topNodes[i - 1];
        }
        topNodes[insertPos] = node;
    }
}
