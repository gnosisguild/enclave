// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";

contract CommitteeSortition {
    // ============ Config ============
    uint256 public immutable submissionWindow; // seconds
    IBondingRegistry public immutable bondingRegistry;
    address public immutable ciphernodeRegistry; // who opens rounds

    // ============ Types ============
    struct Round {
        // lifecycle
        bool initialized;
        bool finalized;
        uint64 startTime;
        uint64 endTime;
        // params
        uint32 committeeSize;
        uint256 seed;
        // state
        address[] topNodes; // sorted by ascending score
        address[] committee; // frozen after finalize
        mapping(address => bool) submitted;
        mapping(address => uint256) scoreOf; // score for submitted node (lower is better)
    }

    // e3Id => Round
    mapping(uint256 => Round) private rounds;

    // ============ Errors ============
    error OnlyCiphernodeRegistry();
    error RoundNotInitialized();
    error RoundAlreadyInitialized();
    error SubmissionWindowClosed();
    error SubmissionWindowNotClosed();
    error NodeNotEligible();
    error NodeAlreadySubmitted();
    error CommitteeAlreadyFinalized();

    // ============ Events ============
    event SortitionInitialized(
        uint256 indexed e3Id,
        uint32 committeeSize,
        uint256 seed,
        uint64 startTime,
        uint64 endTime
    );

    event TicketSubmitted(
        uint256 indexed e3Id,
        address indexed node,
        uint256 ticketId,
        uint256 score
    );

    event CommitteeFinalized(uint256 indexed e3Id, address[] committee);

    // ============ Constructor ============
    constructor(
        address _ciphernodeRegistry,
        address _bondingRegistry,
        uint256 _submissionWindowSeconds
    ) {
        require(_ciphernodeRegistry != address(0), "bad registry");
        require(_bondingRegistry != address(0), "bad bonding");
        require(_submissionWindowSeconds > 0, "bad window");
        ciphernodeRegistry = _ciphernodeRegistry;
        bondingRegistry = IBondingRegistry(_bondingRegistry);
        submissionWindow = _submissionWindowSeconds;
    }

    // ============ View helpers ============
    function getCommittee(
        uint256 e3Id
    ) external view returns (address[] memory) {
        return rounds[e3Id].committee;
    }

    function getTopSoFar(
        uint256 e3Id
    ) external view returns (address[] memory) {
        return rounds[e3Id].topNodes;
    }

    function isOpen(uint256 e3Id) public view returns (bool) {
        Round storage r = rounds[e3Id];
        if (!r.initialized || r.finalized) return false;
        return block.timestamp <= r.endTime;
    }

    // ============ Core ============

    /// called by CiphernodeRegistry when Enclave.requestCommittee(...) happens
    function initializeSortition(
        uint256 e3Id,
        uint32 committeeSize,
        uint256 seed,
        uint256 /* requestBlock */ // kept for compatibility / auditing, not used here
    ) external {
        if (msg.sender != ciphernodeRegistry) revert OnlyCiphernodeRegistry();
        Round storage r = rounds[e3Id];
        if (r.initialized) revert RoundAlreadyInitialized();

        r.initialized = true;
        r.finalized = false;
        r.committeeSize = committeeSize;
        r.seed = seed;
        r.startTime = uint64(block.timestamp);
        r.endTime = uint64(block.timestamp + submissionWindow);

        emit SortitionInitialized(
            e3Id,
            committeeSize,
            seed,
            r.startTime,
            r.endTime
        );
    }

    /// nodes submit their *best* ticket (id, score) if eligible
    function submitTicket(
        uint256 e3Id,
        uint256 ticketId,
        uint256 score
    ) external {
        Round storage r = rounds[e3Id];
        if (!r.initialized) revert RoundNotInitialized();
        if (!isOpen(e3Id)) revert SubmissionWindowClosed();
        if (!IBondingRegistry(bondingRegistry).isActive(msg.sender))
            revert NodeNotEligible();
        if (r.submitted[msg.sender]) revert NodeAlreadySubmitted();

        r.submitted[msg.sender] = true;
        r.scoreOf[msg.sender] = score;

        // insert into top-N (ascending score)
        _insertTopN(r, msg.sender, score);

        emit TicketSubmitted(e3Id, msg.sender, ticketId, score);
    }

    /// anyone can finalize after the window closes
    function finalizeCommittee(uint256 e3Id) external {
        Round storage r = rounds[e3Id];
        if (!r.initialized) revert RoundNotInitialized();
        if (isOpen(e3Id)) revert SubmissionWindowNotClosed();
        if (r.finalized) revert CommitteeAlreadyFinalized();

        r.finalized = true;

        // freeze committee
        uint256 n = r.topNodes.length;
        address[] memory committee = new address[](n);
        for (uint256 i = 0; i < n; i++) committee[i] = r.topNodes[i];
        r.committee = committee;

        emit CommitteeFinalized(e3Id, committee);
    }

    // ============ Internal ============

    function _insertTopN(
        Round storage r,
        address node,
        uint256 score
    ) internal {
        uint256 n = r.topNodes.length;

        // if we still have room, just insert in sorted spot
        if (n < r.committeeSize) {
            uint256 pos = _findInsertPos(r, score);
            r.topNodes.push(node);
            for (uint256 i = r.topNodes.length - 1; i > pos; i--) {
                r.topNodes[i] = r.topNodes[i - 1];
            }
            r.topNodes[pos] = node;
            return;
        }

        // otherwise compare with worst current score
        address worst = r.topNodes[n - 1];
        uint256 worstScore = r.scoreOf[worst];
        if (score >= worstScore) {
            // not better than worst, ignore
            return;
        }

        // replace worst with node at correct position
        r.topNodes.pop(); // drop worst
        uint256 pos2 = _findInsertPos(r, score);
        r.topNodes.push(node);
        for (uint256 i = r.topNodes.length - 1; i > pos2; i--) {
            r.topNodes[i] = r.topNodes[i - 1];
        }
        r.topNodes[pos2] = node;
    }

    function _findInsertPos(
        Round storage r,
        uint256 score
    ) internal view returns (uint256) {
        uint256 n = r.topNodes.length;
        for (uint256 i = 0; i < n; i++) {
            address a = r.topNodes[i];
            if (score < r.scoreOf[a]) return i;
        }
        return n;
    }
}
