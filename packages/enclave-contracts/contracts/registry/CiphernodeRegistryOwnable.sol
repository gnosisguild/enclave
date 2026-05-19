// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";
import { E3 } from "../interfaces/IE3.sol";
import { IEnclave } from "../interfaces/IEnclave.sol";
import { ISlashingManager } from "../interfaces/ISlashingManager.sol";
import {
    Ownable2StepUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/Ownable2StepUpgradeable.sol";
import {
    ReentrancyGuardUpgradeable
} from "@openzeppelin/contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
import {
    InternalLazyIMT,
    LazyIMTData
} from "@zk-kit/lazy-imt.sol/InternalLazyIMT.sol";
import {
    IERC165
} from "@openzeppelin/contracts/utils/introspection/IERC165.sol";

/**
 * @title CiphernodeRegistryOwnable
 * @notice Ownable implementation of the ciphernode registry with IMT-based membership tracking
 * @dev Manages ciphernode registration, committee selection, and integrates with bonding registry
 */
contract CiphernodeRegistryOwnable is
    ICiphernodeRegistry,
    Ownable2StepUpgradeable,
    ReentrancyGuardUpgradeable
{
    using InternalLazyIMT for LazyIMTData;

    /// @notice Thrown when {renounceOwnership} is called.
    error RenounceOwnershipDisabled();

    /// @notice Minimum permitted value for {sortitionSubmissionWindow}.
    uint256 public constant MIN_SORTITION_SUBMISSION_WINDOW = 1;

    /// @notice Maximum permitted value for {sortitionSubmissionWindow}.
    uint256 public constant MAX_SORTITION_SUBMISSION_WINDOW = 7 days;

    /// @notice Thrown when {setSortitionSubmissionWindow} input is outside the
    ///         permitted window.
    error SortitionSubmissionWindowOutOfBounds(uint256 window);

    /// @notice Emitted whenever {slashingManager} is updated.
    event RegistrySlashingManagerSet(address indexed slashingManager);

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Events                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Emitted when the bonding registry address is set
    /// @param bondingRegistry Address of the bonding registry contract
    event BondingRegistrySet(address indexed bondingRegistry);

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Address of the Enclave contract authorized to request committees
    IEnclave public enclave;

    /// @notice Address of the bonding registry for checking node eligibility
    IBondingRegistry public bondingRegistry;

    /// @notice Current number of registered ciphernodes
    uint256 public numCiphernodes;

    /// @notice Submission Window for an E3 Sortition.
    /// @dev The submission window is the time period during which the ciphernodes can submit
    /// their tickets to be a part of the committee.
    uint256 public sortitionSubmissionWindow;

    /// @notice Depth of the LazyIMT tree
    uint8 public constant TREE_DEPTH = 20;

    /// @notice Maximum number of leaves the underlying LazyIMT can hold.
    /// @dev Slots freed by {removeCiphernode} are NOT reused (`_update(0, index)` zeroes
    ///      the slot but never decrements the leaf count), so {addCiphernode} reverts
    ///      with {CiphernodeTreeExhausted} once `numberOfLeaves` reaches this cap.
    uint256 public constant MAX_CIPHERNODE_LEAVES = uint256(1) << TREE_DEPTH;

    /// @notice Thrown when {addCiphernode} would push the LazyIMT past its
    ///         configured {TREE_DEPTH} capacity.
    error CiphernodeTreeExhausted();

    /// @notice Incremental Merkle Tree (IMT) containing all registered ciphernodes
    LazyIMTData public ciphernodes;

    /// @notice Tracks whether a ciphernode is enabled in the registry
    mapping(address node => bool enabled) public ciphernodeEnabled;

    /// @notice Tracks the tree leaf index for each ciphernode
    mapping(address node => uint40 index) public ciphernodeTreeIndex;

    /// @notice Maps E3 ID to the IMT root at the time of committee request
    mapping(uint256 e3Id => uint256 root) public roots;

    /// @notice Maps E3 ID to the hash of the committee's public key
    mapping(uint256 e3Id => bytes32 publicKeyHash) public publicKeyHashes;

    /// @notice Maps E3 ID to its committee data
    mapping(uint256 e3Id => Committee committee) internal committees;

    /// @notice Address of the slashing manager authorized to expel committee members
    ISlashingManager public slashingManager;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                     Modifiers                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @dev Restricts function access to only the Enclave contract
    modifier onlyEnclave() {
        require(msg.sender == address(enclave), OnlyEnclave());
        _;
    }

    /// @dev Restricts function access to only the bonding registry
    modifier onlyBondingRegistry() {
        require(msg.sender == address(bondingRegistry), OnlyBondingRegistry());
        _;
    }

    /// @dev Restricts function access to owner or bonding registry
    modifier onlyOwnerOrBondingVault() {
        require(
            msg.sender == owner() || msg.sender == address(bondingRegistry),
            NotOwnerOrBondingRegistry()
        );
        _;
    }

    /// @dev Restricts function access to only the slashing manager
    modifier onlySlashingManager() {
        require(msg.sender == address(slashingManager), NotSlashingManager());
        _;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Initialization                       //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Locks the implementation; initialize via the proxy.
    constructor() {
        _disableInitializers();
    }

    /// @notice Initializes the registry contract
    /// @param _owner Address that will own the contract
    /// @param _submissionWindow The submission window for the E3 sortition in seconds
    function initialize(
        address _owner,
        uint256 _submissionWindow
    ) public initializer {
        require(_owner != address(0), ZeroAddress());

        __Ownable_init(msg.sender);
        __ReentrancyGuard_init();
        ciphernodes._init(TREE_DEPTH);
        setSortitionSubmissionWindow(_submissionWindow);
        if (_owner != owner()) _transferOwnership(_owner);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Core Entrypoints                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @inheritdoc ICiphernodeRegistry
    /// @dev Uses numActiveOperators() which checks registered + minimum bond + minimum tickets.
    ///      Between request time and ticket submission, operators may become inactive by losing
    ///      bond or tickets. The check at request time may be stale by the time submitTicket
    ///      is called. This is appropriately conservative — it prevents requesting committees
    ///      when not enough operators are active even at request time.
    function requestCommittee(
        uint256 e3Id,
        uint256 seed,
        uint32[2] calldata threshold
    ) external onlyEnclave returns (bool success) {
        Committee storage c = committees[e3Id];
        require(
            c.stage == ICiphernodeRegistry.CommitteeStage.None,
            CommitteeAlreadyRequested()
        );

        uint256 activeCount = bondingRegistry.numActiveOperators();
        require(
            threshold[1] <= activeCount,
            InsufficientCiphernodes(threshold[1], activeCount)
        );

        c.stage = ICiphernodeRegistry.CommitteeStage.Requested;
        c.seed = seed;
        // NOTE: `requestBlock` stores a timepoint per EIP-6372 (mode=timestamp) — its name
        // is kept for storage/event compatibility but it must be compared to
        // {block.timestamp}. This matches the EnclaveTicketToken's timestamp-mode clock so
        // {getPastVotes} lookups resolve consistently.
        c.requestBlock = block.timestamp;
        c.committeeDeadline = block.timestamp + sortitionSubmissionWindow;
        c.threshold = threshold;
        roots[e3Id] = root();

        emit CommitteeRequested(
            e3Id,
            seed,
            threshold,
            c.requestBlock,
            c.committeeDeadline
        );
        success = true;
    }

    /// @inheritdoc ICiphernodeRegistry
    function publishCommittee(
        uint256 e3Id,
        address[] calldata nodes,
        bytes calldata publicKey,
        bytes32 pkCommitment,
        bytes calldata proof
    ) external nonReentrant {
        Committee storage c = committees[e3Id];

        require(
            c.stage == ICiphernodeRegistry.CommitteeStage.Finalized,
            CommitteeNotFinalized()
        );
        require(c.publicKey == bytes32(0), CommitteeAlreadyPublished());
        require(nodes.length == c.topNodes.length, "Node count mismatch");
        require(pkCommitment != bytes32(0), "pkCommitment required");

        E3 memory e3 = enclave.getE3(e3Id);
        if (e3.proofAggregationEnabled) {
            require(proof.length > 0, "proof required");
            // Wrapper binds proof to full call context (chainId, this, e3Id, committeeRoot,
            // sortedNodes, pkCommitment) and anchors recursive-aggregation VKs against
            // immutables; reverts on mismatch with a typed error. Bind to the on-chain
            // selected committee (`c.topNodes`), not caller-supplied `nodes`, so a wrong
            // `nodes` input cannot pre-commit the prover to the attacker's set.
            e3.pkVerifier.verify(
                e3Id,
                roots[e3Id],
                c.topNodes,
                pkCommitment,
                proof
            );
        }

        c.publicKey = pkCommitment;
        publicKeyHashes[e3Id] = pkCommitment;

        enclave.onCommitteePublished(e3Id, pkCommitment);

        emit CommitteePublished(e3Id, nodes, publicKey, pkCommitment, proof);
    }

    /// @inheritdoc ICiphernodeRegistry
    function addCiphernode(address node) external onlyOwnerOrBondingVault {
        if (isEnabled(node)) {
            return;
        }

        uint40 index = ciphernodes.numberOfLeaves;
        // cap insertions before LazyIMT depth is exhausted. Slots
        // freed by {removeCiphernode} are not reclaimed, so monotonic
        // register/deregister churn would otherwise brick the registry.
        require(
            uint256(index) < MAX_CIPHERNODE_LEAVES,
            CiphernodeTreeExhausted()
        );
        ciphernodes._insert(uint160(node));
        ciphernodeEnabled[node] = true;
        ciphernodeTreeIndex[node] = index;
        numCiphernodes++;
        emit CiphernodeAdded(
            node,
            index,
            numCiphernodes,
            ciphernodes.numberOfLeaves
        );
    }

    /// @inheritdoc ICiphernodeRegistry
    function removeCiphernode(address node) external onlyOwnerOrBondingVault {
        require(isEnabled(node), CiphernodeNotEnabled(node));

        uint40 index = ciphernodeTreeIndex[node];
        ciphernodes._update(0, index);
        ciphernodeEnabled[node] = false;
        numCiphernodes--;
        emit CiphernodeRemoved(
            node,
            index,
            numCiphernodes,
            ciphernodes.numberOfLeaves
        );
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Sortition Functions                  //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Submit a ticket for sortition
    /// @dev Validates ticket against node's balance at request block and inserts into top-N
    /// @param e3Id ID of the E3 computation
    /// @param ticketNumber The ticket number to submit (1 to available tickets at snapshot)
    function submitTicket(uint256 e3Id, uint256 ticketNumber) external {
        Committee storage c = committees[e3Id];
        require(
            c.stage != ICiphernodeRegistry.CommitteeStage.None,
            CommitteeNotRequested()
        );
        require(
            c.stage == ICiphernodeRegistry.CommitteeStage.Requested,
            CommitteeAlreadyFinalized()
        );
        require(
            block.timestamp <= c.committeeDeadline,
            CommitteeDeadlineReached()
        );
        require(!c.submitted[msg.sender], NodeAlreadySubmitted());
        require(isCiphernodeEligible(msg.sender), NodeNotEligible());

        // Validate node eligibility and ticket number
        _validateNodeEligibility(msg.sender, ticketNumber, e3Id);

        // Compute score using the seed committed at request time. Same-block
        // manipulation is bounded by the snapshot of ticket balances at
        // `c.requestBlock - 1` performed inside {_validateNodeEligibility}.
        uint256 score = _computeTicketScore(
            msg.sender,
            ticketNumber,
            e3Id,
            c.seed
        );

        // Store submission
        c.submitted[msg.sender] = true;

        // Insert into top-N (ascending score)
        _insertTopN(c, msg.sender, score);

        emit TicketSubmitted(e3Id, msg.sender, ticketNumber, score);
    }

    /// @notice Finalize the committee after submission window closes
    /// @dev Can be called by anyone after the deadline. If threshold not met, marks E3 as failed.
    /// @param e3Id ID of the E3 computation
    /// @return success True if committee formed successfully, false if threshold not met
    function finalizeCommittee(
        uint256 e3Id
    ) external nonReentrant returns (bool success) {
        Committee storage c = committees[e3Id];
        require(
            c.stage != ICiphernodeRegistry.CommitteeStage.None,
            CommitteeNotRequested()
        );
        require(
            c.stage == ICiphernodeRegistry.CommitteeStage.Requested,
            CommitteeAlreadyFinalized()
        );
        require(
            block.timestamp > c.committeeDeadline,
            SubmissionWindowNotClosed()
        );
        bool thresholdMet = c.topNodes.length >= c.threshold[1];

        if (!thresholdMet) {
            c.stage = ICiphernodeRegistry.CommitteeStage.Failed;
            emit CommitteeFormationFailed(
                e3Id,
                c.topNodes.length,
                c.threshold[1]
            );
            enclave.onE3Failed(
                e3Id,
                uint8(IEnclave.FailureReason.InsufficientCommitteeMembers)
            );
            return false;
        }

        c.stage = ICiphernodeRegistry.CommitteeStage.Finalized;
        c.activeCount = c.topNodes.length;

        uint256 len = c.topNodes.length;
        uint256[] memory scores = new uint256[](len);
        for (uint256 i = 0; i < len; ++i) {
            scores[i] = c.scoreOf[c.topNodes[i]];
        }

        enclave.onCommitteeFinalized(e3Id);
        emit SortitionCommitteeFinalized(e3Id, c.topNodes, scores);
        return true;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Set Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Sets the Enclave contract address
    /// @dev Only callable by owner
    /// @param _enclave Address of the Enclave contract
    function setEnclave(IEnclave _enclave) public onlyOwner {
        require(address(_enclave) != address(0), ZeroAddress());
        enclave = _enclave;
        emit EnclaveSet(address(_enclave));
    }

    /// @notice Sets the bonding registry contract address
    /// @dev Only callable by owner
    /// @param _bondingRegistry Address of the bonding registry contract
    function setBondingRegistry(
        IBondingRegistry _bondingRegistry
    ) public onlyOwner {
        require(address(_bondingRegistry) != address(0), ZeroAddress());
        bondingRegistry = _bondingRegistry;
        emit BondingRegistrySet(address(_bondingRegistry));
    }

    /// @notice Sets the slashing manager contract address
    /// @dev Only callable by owner
    /// @param _slashingManager Address of the slashing manager contract
    function setSlashingManager(
        ISlashingManager _slashingManager
    ) public onlyOwner {
        require(address(_slashingManager) != address(0), ZeroAddress());
        slashingManager = _slashingManager;
        emit RegistrySlashingManagerSet(address(_slashingManager));
    }

    /// @notice Disabled. Reverts unconditionally.
    function renounceOwnership() public view override onlyOwner {
        revert RenounceOwnershipDisabled();
    }

    /// @inheritdoc ICiphernodeRegistry
    function setSortitionSubmissionWindow(
        uint256 _sortitionSubmissionWindow
    ) public onlyOwner {
        require(
            _sortitionSubmissionWindow >= MIN_SORTITION_SUBMISSION_WINDOW &&
                _sortitionSubmissionWindow <= MAX_SORTITION_SUBMISSION_WINDOW,
            SortitionSubmissionWindowOutOfBounds(_sortitionSubmissionWindow)
        );
        sortitionSubmissionWindow = _sortitionSubmissionWindow;
        emit SortitionSubmissionWindowSet(_sortitionSubmissionWindow);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Check if submission window is still open for an E3
    /// @param e3Id ID of the E3 computation
    /// @return Whether the submission window is open
    function isOpen(uint256 e3Id) public view returns (bool) {
        Committee storage c = committees[e3Id];
        if (c.stage != ICiphernodeRegistry.CommitteeStage.Requested)
            return false;
        return block.timestamp <= c.committeeDeadline;
    }

    /// @inheritdoc ICiphernodeRegistry
    function committeePublicKey(
        uint256 e3Id
    ) external view returns (bytes32 publicKeyHash) {
        publicKeyHash = publicKeyHashes[e3Id];
        require(publicKeyHash != bytes32(0), CommitteeNotPublished());
    }

    /// @inheritdoc ICiphernodeRegistry
    function isCiphernodeEligible(address node) public view returns (bool) {
        if (!isEnabled(node)) return false;

        require(
            address(bondingRegistry) != address(0),
            BondingRegistryNotSet()
        );
        return bondingRegistry.isActive(node);
    }

    /// @inheritdoc ICiphernodeRegistry
    function isEnabled(address node) public view returns (bool) {
        return ciphernodeEnabled[node];
    }

    /// @notice Returns the current root of the ciphernode IMT
    /// @return Current IMT root
    function root() public view returns (uint256) {
        return ciphernodes._root(TREE_DEPTH);
    }

    /// @notice Returns the IMT root at the time a committee was requested
    /// @param e3Id ID of the E3
    /// @return IMT root at time of committee request
    function rootAt(uint256 e3Id) public view returns (uint256) {
        return roots[e3Id];
    }

    /// @inheritdoc ICiphernodeRegistry
    function getCommitteeNodes(
        uint256 e3Id
    ) public view returns (address[] memory nodes) {
        Committee storage c = committees[e3Id];
        require(c.publicKey != bytes32(0), CommitteeNotPublished());
        nodes = c.topNodes;
    }

    /// @notice Returns the current size of the ciphernode IMT
    /// @return Size of the IMT
    function treeSize() public view returns (uint256) {
        return ciphernodes.numberOfLeaves;
    }

    /// @notice Returns the address of the bonding registry
    /// @return Address of the bonding registry contract
    function getBondingRegistry() external view returns (address) {
        return address(bondingRegistry);
    }

    /// @inheritdoc ICiphernodeRegistry
    function getCommitteeDeadline(
        uint256 e3Id
    ) external view returns (uint256) {
        Committee storage c = committees[e3Id];
        require(
            c.stage != ICiphernodeRegistry.CommitteeStage.None,
            CommitteeNotRequested()
        );
        return c.committeeDeadline;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //              Committee Expulsion Functions             //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @inheritdoc ICiphernodeRegistry
    function expelCommitteeMember(
        uint256 e3Id,
        address node,
        bytes32 reason
    )
        external
        onlySlashingManager
        returns (uint256 activeCount, uint32 thresholdM)
    {
        Committee storage c = committees[e3Id];
        require(
            c.stage == ICiphernodeRegistry.CommitteeStage.Finalized,
            CommitteeNotFinalized()
        );
        thresholdM = c.threshold[0];

        // Idempotent: if already expelled (or never a member), return current state
        if (c.memberStatus[node] != ICiphernodeRegistry.MemberStatus.Active) {
            activeCount = c.activeCount;
            return (activeCount, thresholdM);
        }

        c.memberStatus[node] = ICiphernodeRegistry.MemberStatus.Expelled;
        c.activeCount--;

        activeCount = c.activeCount;
        emit CommitteeMemberExpelled(e3Id, node, reason, activeCount);

        // Emit viability update
        bool viable = activeCount >= thresholdM;
        emit CommitteeViabilityUpdated(e3Id, activeCount, thresholdM, viable);
    }

    /// @inheritdoc ICiphernodeRegistry
    function isCommitteeMemberActive(
        uint256 e3Id,
        address node
    ) external view returns (bool) {
        return
            committees[e3Id].memberStatus[node] ==
            ICiphernodeRegistry.MemberStatus.Active;
    }

    /// @inheritdoc ICiphernodeRegistry
    function isCommitteeMember(
        uint256 e3Id,
        address node
    ) external view returns (bool) {
        return
            committees[e3Id].memberStatus[node] !=
            ICiphernodeRegistry.MemberStatus.None;
    }

    /// @inheritdoc ICiphernodeRegistry
    function getActiveCommitteeNodes(
        uint256 e3Id
    ) external view returns (address[] memory nodes, uint256[] memory scores) {
        Committee storage c = committees[e3Id];
        uint256 total = c.topNodes.length;
        uint256 actCount = c.activeCount;

        nodes = new address[](actCount);
        scores = new uint256[](actCount);
        uint256 idx = 0;
        for (uint256 i = 0; i < total; ++i) {
            address node = c.topNodes[i];
            if (
                c.memberStatus[node] == ICiphernodeRegistry.MemberStatus.Active
            ) {
                nodes[idx] = node;
                scores[idx] = c.scoreOf[node];
                idx++;
            }
        }
    }

    /// @inheritdoc ICiphernodeRegistry
    function getCommitteeViability(
        uint256 e3Id
    )
        external
        view
        returns (
            uint256 activeCount,
            uint32 thresholdM,
            uint32 thresholdN,
            bool viable
        )
    {
        Committee storage c = committees[e3Id];
        activeCount = c.activeCount;
        thresholdM = c.threshold[0];
        thresholdN = c.threshold[1];
        viable = activeCount >= thresholdM;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Internal Functions                   //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Computes ticket score as keccak256(node || ticketNumber || e3Id || seed)
    /// @param node Address of the ciphernode
    /// @param ticketNumber The ticket number
    /// @param e3Id ID of the E3 computation
    /// @param seed Random seed for the E3
    /// @return score The computed score
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

    /// @notice Validates that a node is eligible to submit a ticket
    /// @dev Uses snapshot of ticket balance at (requestBlock - 1) for deterministic validation.
    ///      The -1 offset prevents same-block manipulation attacks where an operator could deposit
    ///      tickets and submit in the same transaction. Deposits in the request block itself are
    ///      excluded. This is conservative but not fully settled — see TODO below.
    /// @param node Address of the ciphernode
    /// @param ticketNumber The ticket number being submitted
    /// @param e3Id ID of the E3 computation
    function _validateNodeEligibility(
        address node,
        uint256 ticketNumber,
        uint256 e3Id
    ) internal view {
        require(ticketNumber > 0, InvalidTicketNumber());
        require(
            address(bondingRegistry) != address(0),
            BondingRegistryNotSet()
        );

        Committee storage c = committees[e3Id];

        // bind ticket weight to the request-time snapshot via the
        // ticket token's EIP-6372 ERC20Votes checkpoints. The outer
        // `isCiphernodeEligible(msg.sender)` check in {submitTicket} still
        // gates on the operator's *current* `isActive` flag, but the score
        // and selection weight below derive purely from the historical
        // ticket balance at `c.requestBlock - 1`, so churn between request
        // time and the ticket submission window cannot inflate weights.
        uint256 ticketBalance = bondingRegistry.getTicketBalanceAtBlock(
            node,
            c.requestBlock - 1
        );
        uint256 ticketPrice = bondingRegistry.ticketPrice();

        require(ticketPrice > 0, InvalidTicketNumber());
        uint256 availableTickets = ticketBalance / ticketPrice;

        require(availableTickets > 0, NodeNotEligible());
        require(ticketNumber <= availableTickets, InvalidTicketNumber());
    }

    /// @notice Inserts a node into the top-N list - Smallest scores
    /// @dev O(N) linear scan per insertion to find the worst score. For a committee of size N
    ///      with S total submissions, total gas is O(N * S). With N=20 and S=1000, this is ~20K
    ///      iterations at ~200 gas each (≈ 4M gas total), which is acceptable for current
    ///      parameters. Will not scale to N > ~50 without switching to a heap or sorted
    ///      data structure.
    /// @param c Committee storage reference
    /// @param node Address of the node
    /// @param score Score of the node
    /// @return entered Whether the node was inserted into the top-N
    function _insertTopN(
        Committee storage c,
        address node,
        uint256 score
    ) internal returns (bool entered) {
        address[] storage top = c.topNodes;
        uint256 cap = c.threshold[1];

        if (top.length < cap) {
            top.push(node);
            c.scoreOf[node] = score;
            c.memberStatus[node] = ICiphernodeRegistry.MemberStatus.Active;
            return true;
        }

        uint256 worstIdx = 0;
        uint256 worstScore = c.scoreOf[top[0]];
        for (uint256 i = 1; i < top.length; ++i) {
            uint256 s = c.scoreOf[top[i]];
            if (s > worstScore) {
                worstScore = s;
                worstIdx = i;
            }
        }

        if (score >= worstScore) return false;

        c.memberStatus[top[worstIdx]] = ICiphernodeRegistry.MemberStatus.None;
        top[worstIdx] = node;
        c.scoreOf[node] = score;
        c.memberStatus[node] = ICiphernodeRegistry.MemberStatus.Active;

        return true;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //              ERC-165 Interface Detection               //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice ERC-165 interface detection. Advertises
    ///         {ICiphernodeRegistry} and {IERC165}.
    function supportsInterface(
        bytes4 interfaceId
    ) external pure virtual returns (bool) {
        return
            interfaceId == type(ICiphernodeRegistry).interfaceId ||
            interfaceId == type(IERC165).interfaceId;
    }

    /// @dev Reserved storage slots for future upgrades.
    // solhint-disable-next-line var-name-mixedcase
    uint256[50] private __gap;
}
