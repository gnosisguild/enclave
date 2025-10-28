// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {
    InternalLeanIMT,
    LeanIMTData
} from "@zk-kit/lean-imt.sol/InternalLeanIMT.sol";

/**
 * @title CiphernodeRegistryOwnable
 * @notice Ownable implementation of the ciphernode registry with IMT-based membership tracking
 * @dev Manages ciphernode registration, committee selection, and integrates with bonding registry
 */
contract CiphernodeRegistryOwnable is ICiphernodeRegistry, OwnableUpgradeable {
    using InternalLeanIMT for LeanIMTData;

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
    address public enclave;

    /// @notice Address of the bonding registry for checking node eligibility
    address public bondingRegistry;

    /// @notice Current number of registered ciphernodes
    uint256 public numCiphernodes;

    /// @notice Submission Window for an E3 Sortition.
    /// @dev The submission window is the time period during which the ciphernodes can submit
    /// their tickets to be a part of the committee.
    uint256 public sortitionSubmissionWindow;

    /// @notice Incremental Merkle Tree (IMT) containing all registered ciphernodes
    LeanIMTData public ciphernodes;

    /// @notice Maps E3 ID to the IMT root at the time of committee request
    mapping(uint256 e3Id => uint256 root) public roots;

    /// @notice Maps E3 ID to the hash of the committee's public key
    mapping(uint256 e3Id => bytes32 publicKeyHash) public publicKeyHashes;

    /// @notice Maps E3 ID to its committee data
    mapping(uint256 e3Id => Committee committee) internal committees;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Committee has already been requested for this E3
    error CommitteeAlreadyRequested();

    /// @notice Committee has already been published for this E3
    error CommitteeAlreadyPublished();

    /// @notice Committee has not been published yet for this E3
    error CommitteeNotPublished();

    /// @notice Committee has not been requested yet for this E3
    error CommitteeNotRequested();

    /// @notice Committee Not Initialized or Finalized
    error CommitteeNotInitializedOrFinalized();

    /// @notice Submission Window has been closed for this E3
    error SubmissionWindowClosed();

    /// @notice Submission deadline has been reached for this E3
    error SubmissionDeadlineReached();

    /// @notice Committee has already been finalized for this E3
    error CommitteeAlreadyFinalized();

    /// @notice Committee has not been finalized yet for this E3
    error CommitteeNotFinalized();

    /// @notice Node has already submitted a ticket for this E3
    error NodeAlreadySubmitted();

    /// @notice Node has not submitted a ticket for this E3
    error NodeNotSubmitted();

    /// @notice Node is not eligible for this E3
    error NodeNotEligible();

    /// @notice Ciphernode is not enabled in the registry
    /// @param node Address of the ciphernode
    error CiphernodeNotEnabled(address node);

    /// @notice Caller is not the Enclave contract
    error OnlyEnclave();

    /// @notice Caller is not the bonding registry
    error OnlyBondingRegistry();

    /// @notice Caller is neither owner nor bonding registry
    error NotOwnerOrBondingRegistry();

    /// @notice Node is not bonded
    /// @param node Address of the node
    error NodeNotBonded(address node);

    /// @notice Address cannot be zero
    error ZeroAddress();

    /// @notice Bonding registry has not been set
    error BondingRegistryNotSet();

    /// @notice Invalid ticket number
    error InvalidTicketNumber();

    /// @notice Submission window not closed yet
    error SubmissionWindowNotClosed();

    /// @notice Threshold not met for this E3
    error ThresholdNotMet();

    /// @notice Caller is not authorized
    error Unauthorized();

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                     Modifiers                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @dev Restricts function access to only the Enclave contract
    modifier onlyEnclave() {
        require(msg.sender == enclave, OnlyEnclave());
        _;
    }

    /// @dev Restricts function access to only the bonding registry
    modifier onlyBondingRegistry() {
        require(msg.sender == bondingRegistry, OnlyBondingRegistry());
        _;
    }

    /// @dev Restricts function access to owner or bonding registry
    modifier onlyOwnerOrBondingVault() {
        require(
            msg.sender == owner() || msg.sender == bondingRegistry,
            NotOwnerOrBondingRegistry()
        );
        _;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Initialization                       //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Constructor that initializes the registry with owner and enclave
    /// @param _owner Address that will own the contract
    /// @param _enclave Address of the Enclave contract
    /// @param _submissionWindow The submission window for the E3 sortition in seconds
    constructor(address _owner, address _enclave, uint256 _submissionWindow) {
        initialize(_owner, _enclave, _submissionWindow);
    }

    /// @notice Initializes the registry contract
    /// @dev Can only be called once due to initializer modifier
    /// @param _owner Address that will own the contract
    /// @param _enclave Address of the Enclave contract
    /// @param _submissionWindow The submission window for the E3 sortition in seconds
    function initialize(
        address _owner,
        address _enclave,
        uint256 _submissionWindow
    ) public initializer {
        require(_owner != address(0), ZeroAddress());
        require(_enclave != address(0), ZeroAddress());

        __Ownable_init(msg.sender);
        setEnclave(_enclave);
        setSortitionSubmissionWindow(_submissionWindow);
        if (_owner != owner()) transferOwnership(_owner);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Core Entrypoints                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @inheritdoc ICiphernodeRegistry
    function requestCommittee(
        uint256 e3Id,
        uint256 seed,
        uint32[2] calldata threshold
    ) external onlyEnclave returns (bool success) {
        Committee storage c = committees[e3Id];
        require(!c.initialized, CommitteeAlreadyRequested());

        c.initialized = true;
        c.finalized = false;
        c.seed = seed;
        c.requestBlock = block.number;
        c.submissionDeadline = block.timestamp + sortitionSubmissionWindow;
        c.threshold = threshold;
        roots[e3Id] = root();

        emit CommitteeRequested(
            e3Id,
            seed,
            threshold,
            c.requestBlock,
            c.submissionDeadline
        );
        success = true;
    }

    /// @notice Publishes a committee for an E3 computation
    /// @dev Only callable by owner. Verifies committee is finalized and matches provided nodes.
    /// @param e3Id ID of the E3 computation
    /// @param nodes Array of ciphernode addresses selected for the committee
    /// @param publicKey Aggregated public key of the committee
    function publishCommittee(
        uint256 e3Id,
        address[] calldata nodes,
        bytes calldata publicKey
    ) external onlyOwner {
        Committee storage c = committees[e3Id];

        require(c.initialized, CommitteeNotRequested());
        require(c.finalized, CommitteeNotFinalized());
        require(c.publicKey == bytes32(0), CommitteeAlreadyPublished());
        require(nodes.length == c.committee.length, "Node count mismatch");

        // TODO: Currently we trust the owner to publish the correct committee.
        // TODO: Need a Proof that the public key is generated from the committee
        bytes32 publicKeyHash = keccak256(publicKey);
        c.publicKey = publicKeyHash;
        publicKeyHashes[e3Id] = publicKeyHash;
        emit CommitteePublished(e3Id, nodes, publicKey);
    }

    /// @inheritdoc ICiphernodeRegistry
    function addCiphernode(address node) external onlyOwnerOrBondingVault {
        if (isEnabled(node)) {
            return;
        }

        uint160 ciphernode = uint160(node);
        ciphernodes._insert(ciphernode);
        numCiphernodes++;
        emit CiphernodeAdded(
            node,
            ciphernodes._indexOf(ciphernode),
            numCiphernodes,
            ciphernodes.size
        );
    }

    /// @inheritdoc ICiphernodeRegistry
    function removeCiphernode(
        address node,
        uint256[] calldata siblingNodes
    ) external onlyOwnerOrBondingVault {
        require(isEnabled(node), CiphernodeNotEnabled(node));

        uint160 ciphernode = uint160(node);
        uint256 index = ciphernodes._indexOf(ciphernode);
        ciphernodes._remove(ciphernode, siblingNodes);
        numCiphernodes--;
        emit CiphernodeRemoved(node, index, numCiphernodes, ciphernodes.size);
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
        require(c.initialized, CommitteeNotRequested());
        require(!c.finalized, CommitteeAlreadyFinalized());
        require(
            block.timestamp <= c.submissionDeadline,
            SubmissionDeadlineReached()
        );
        require(!c.submitted[msg.sender], NodeAlreadySubmitted());

        // Validate node eligibility and ticket number
        _validateNodeEligibility(msg.sender, ticketNumber, e3Id);

        // Compute score
        uint256 score = _computeTicketScore(
            msg.sender,
            ticketNumber,
            e3Id,
            c.seed
        );

        // Store submission
        c.submitted[msg.sender] = true;
        c.scoreOf[msg.sender] = score;

        // Insert into top-N (ascending score)
        _insertTopN(c, msg.sender, score);

        emit TicketSubmitted(e3Id, msg.sender, ticketNumber, score);
    }

    /// @notice Finalize the committee after submission window closes
    /// @dev Can be called by anyone after the deadline. Reverts if not enough nodes submitted.
    /// @param e3Id ID of the E3 computation
    function finalizeCommittee(uint256 e3Id) external {
        Committee storage c = committees[e3Id];
        require(c.initialized, CommitteeNotRequested());
        require(!c.finalized, CommitteeAlreadyFinalized());
        require(
            block.timestamp >= c.submissionDeadline,
            SubmissionWindowNotClosed()
        );
        // TODO: Handle what happens if the threshold is not met.
        require(c.topNodes.length >= c.threshold[0], ThresholdNotMet());

        c.finalized = true;
        c.committee = c.topNodes;

        emit CommitteeFinalized(e3Id, c.topNodes);
    }

    /// @notice Check if submission window is still open for an E3
    /// @param e3Id ID of the E3 computation
    /// @return Whether the submission window is open
    function isOpen(uint256 e3Id) public view returns (bool) {
        Committee storage c = committees[e3Id];
        if (!c.initialized || c.finalized) return false;
        return block.timestamp <= c.submissionDeadline;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Set Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Sets the Enclave contract address
    /// @dev Only callable by owner
    /// @param _enclave Address of the Enclave contract
    function setEnclave(address _enclave) public onlyOwner {
        require(_enclave != address(0), ZeroAddress());
        enclave = _enclave;
        emit EnclaveSet(_enclave);
    }

    /// @notice Sets the bonding registry contract address
    /// @dev Only callable by owner
    /// @param _bondingRegistry Address of the bonding registry contract
    function setBondingRegistry(address _bondingRegistry) public onlyOwner {
        require(_bondingRegistry != address(0), ZeroAddress());
        bondingRegistry = _bondingRegistry;
        emit BondingRegistrySet(_bondingRegistry);
    }

    /// @inheritdoc ICiphernodeRegistry
    function setSortitionSubmissionWindow(
        uint256 _sortitionSubmissionWindow
    ) public onlyOwner {
        sortitionSubmissionWindow = _sortitionSubmissionWindow;
        emit SortitionSubmissionWindowSet(_sortitionSubmissionWindow);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @inheritdoc ICiphernodeRegistry
    function committeePublicKey(
        uint256 e3Id
    ) external view returns (bytes32 publicKeyHash) {
        publicKeyHash = publicKeyHashes[e3Id];
        require(publicKeyHash != bytes32(0), CommitteeNotPublished());
    }

    /// @inheritdoc ICiphernodeRegistry
    function isCiphernodeEligible(address node) external view returns (bool) {
        if (!isEnabled(node)) return false;

        require(bondingRegistry != address(0), BondingRegistryNotSet());
        return IBondingRegistry(bondingRegistry).isActive(node);
    }

    /// @inheritdoc ICiphernodeRegistry
    function isEnabled(address node) public view returns (bool) {
        return ciphernodes._has(uint160(node));
    }

    /// @notice Returns the current root of the ciphernode IMT
    /// @return Current IMT root
    function root() public view returns (uint256) {
        return (ciphernodes._root());
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
        nodes = c.committee;
    }

    /// @notice Returns the current size of the ciphernode IMT
    /// @return Size of the IMT
    function treeSize() public view returns (uint256) {
        return ciphernodes.size;
    }

    /// @notice Returns the address of the bonding registry
    /// @return Address of the bonding registry contract
    function getBondingRegistry() external view returns (address) {
        return bondingRegistry;
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
    /// @dev Uses snapshot of ticket balance at E3 request block for deterministic validation
    /// @param node Address of the ciphernode
    /// @param ticketNumber The ticket number being submitted
    /// @param e3Id ID of the E3 computation
    function _validateNodeEligibility(
        address node,
        uint256 ticketNumber,
        uint256 e3Id
    ) internal view {
        require(ticketNumber > 0, InvalidTicketNumber());
        require(bondingRegistry != address(0), BondingRegistryNotSet());

        Committee storage c = committees[e3Id];

        uint256 ticketBalance = IBondingRegistry(bondingRegistry)
            .getTicketBalanceAtBlock(node, c.requestBlock);
        uint256 ticketPrice = IBondingRegistry(bondingRegistry).ticketPrice();

        require(ticketPrice > 0, InvalidTicketNumber());
        uint256 availableTickets = ticketBalance / ticketPrice;

        require(availableTickets > 0, NodeNotEligible());
        require(ticketNumber <= availableTickets, InvalidTicketNumber());
    }

    /// @notice Inserts a node into the top-N sorted list by score
    /// @dev Maintains sorted order (ascending by score)
    /// @param c Committee storage reference
    /// @param node Address of the ciphernode
    /// @param score The computed score
    function _insertTopN(
        Committee storage c,
        address node,
        uint256 score
    ) internal {
        address[] storage topNodes = c.topNodes;

        // If list not full, insert in sorted order
        if (topNodes.length < c.threshold[1]) {
            _insertSorted(c, node, score);
            return;
        }

        // If list is full, only add if score is better than worst
        uint256 worstScore = c.scoreOf[topNodes[topNodes.length - 1]];
        if (score < worstScore) {
            topNodes.pop();
            _insertSorted(c, node, score);
        }
    }

    /// @notice Inserts a node at the correct sorted position (ascending by score)
    /// @param c Committee storage reference
    /// @param node Address of the ciphernode
    /// @param score The computed score
    function _insertSorted(
        Committee storage c,
        address node,
        uint256 score
    ) internal {
        address[] storage topNodes = c.topNodes;

        // Find insertion position
        uint256 insertPos = topNodes.length;
        for (uint256 i = 0; i < topNodes.length; i++) {
            uint256 existingScore = c.scoreOf[topNodes[i]];
            if (score < existingScore) {
                insertPos = i;
                break;
            }
        }

        // Insert at position
        topNodes.push(address(0));
        for (uint256 i = topNodes.length - 1; i > insertPos; i--) {
            topNodes[i] = topNodes[i - 1];
        }
        topNodes[insertPos] = node;
    }
}
