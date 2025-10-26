// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";
import { CommitteeSortition } from "../sortition/CommitteeSortition.sol";
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

    /// @notice Emitted when the committee sortition address is set
    /// @param committeeSortition Address of the committee sortition contract
    event CommitteeSortitionSet(address indexed committeeSortition);

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Address of the Enclave contract authorized to request committees
    address public enclave;

    /// @notice Address of the bonding registry for checking node eligibility
    address public bondingRegistry;

    /// @notice Address of the committee sortition contract
    address public committeeSortition;

    /// @notice Current number of registered ciphernodes
    uint256 public numCiphernodes;

    /// @notice Incremental Merkle Tree (IMT) containing all registered ciphernodes
    LeanIMTData public ciphernodes;

    /// @notice Maps E3 ID to the IMT root at the time of committee request
    mapping(uint256 e3Id => uint256 root) public roots;

    /// @notice Maps E3 ID to the hash of the committee's public key
    mapping(uint256 e3Id => bytes32 publicKeyHash) public publicKeyHashes;

    /// @notice Maps E3 ID to its committee data
    mapping(uint256 e3Id => Committee committee) public committees;

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

    /// @notice Submission Window Not valid for this E3
    error SubmissionWindowNotValid();

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

    /// @notice Committee sortition has not been set
    error CommitteeSortitionNotSet();

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
    constructor(address _owner, address _enclave) {
        initialize(_owner, _enclave);
    }

    /// @notice Initializes the registry contract
    /// @dev Can only be called once due to initializer modifier
    /// @param _owner Address that will own the contract
    /// @param _enclave Address of the Enclave contract
    function initialize(address _owner, address _enclave) public initializer {
        require(_owner != address(0), ZeroAddress());
        require(_enclave != address(0), ZeroAddress());

        __Ownable_init(msg.sender);
        setEnclave(_enclave);
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
        uint32[2] calldata threshold,
        uint256 submissionWindow
    ) external onlyEnclave returns (bool success) {
        Committee storage c = committees[e3Id];
        require(!c.initialized, CommitteeAlreadyRequested());

        c.initialized = true;
        c.finalized = false;
        c.seed = seed;
        c.requestBlock = block.number;
        c.submissionDeadline = block.timestamp + submissionWindow;
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
    /// @dev Only callable by owner. Stores committee data and emits event
    /// @param e3Id ID of the E3 computation
    /// @param nodes Array of ciphernode addresses selected for the committee
    /// @param publicKey Aggregated public key of the committee
    function publishCommittee(
        uint256 e3Id,
        address[] calldata nodes,
        bytes calldata publicKey
    ) external onlyOwner {
        ICiphernodeRegistry.Committee storage committee = committees[e3Id];
        require(committee.publicKey == bytes32(0), CommitteeAlreadyPublished());
        committee.nodes = nodes;
        bytes32 publicKeyHash = keccak256(publicKey);
        committee.publicKey = publicKeyHash;
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

    /// @inheritdoc ICiphernodeRegistry
    function submitTicket(
        uint256 e3Id,
        uint256 ticketId,
        uint256 score
    ) external {
        Committee storage c = committees[e3Id];
        require(!r.initialized || r.finalized, CommitteeNotRequested());
        require(
            block.timestamp <= c.submissionDeadline,
            SubmissionDeadlineReached()
        );

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

    /// @notice Sets the committee sortition contract address
    /// @dev Only callable by owner
    /// @param _committeeSortition Address of the committee sortition contract
    function setCommitteeSortition(
        address _committeeSortition
    ) public onlyOwner {
        require(_committeeSortition != address(0), ZeroAddress());
        committeeSortition = _committeeSortition;
        emit CommitteeSortitionSet(_committeeSortition);
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
    function getCommittee(
        uint256 e3Id
    ) public view returns (ICiphernodeRegistry.Committee memory committee) {
        committee = committees[e3Id];
        require(committee.publicKey != bytes32(0), CommitteeNotPublished());
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
}
