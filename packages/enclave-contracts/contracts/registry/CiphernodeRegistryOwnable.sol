// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IRegistryFilter } from "../interfaces/IRegistryFilter.sol";
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

    /// @notice Incremental Merkle Tree (IMT) containing all registered ciphernodes
    LeanIMTData public ciphernodes;

    /// @notice Maps E3 ID to its associated registry filter contract
    mapping(uint256 e3Id => IRegistryFilter filter) public registryFilters;

    /// @notice Maps E3 ID to the IMT root at the time of committee request
    mapping(uint256 e3Id => uint256 root) public roots;

    /// @notice Maps E3 ID to the hash of the committee's public key
    mapping(uint256 e3Id => bytes32 publicKeyHash) public publicKeyHashes;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Committee has already been requested for this E3
    error CommitteeAlreadyRequested();

    /// @notice Committee has already been published for this E3
    error CommitteeAlreadyPublished();

    /// @notice Caller is not the authorized filter for this E3
    error OnlyFilter();

    /// @notice Committee has not been published yet for this E3
    error CommitteeNotPublished();

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
        address filter,
        uint32[2] calldata threshold
    ) external onlyEnclave returns (bool success) {
        require(
            registryFilters[e3Id] == IRegistryFilter(address(0)),
            CommitteeAlreadyRequested()
        );
        registryFilters[e3Id] = IRegistryFilter(filter);
        roots[e3Id] = root();

        IRegistryFilter(filter).requestCommittee(e3Id, threshold);
        emit CommitteeRequested(e3Id, filter, threshold);
        success = true;
    }

    /// @inheritdoc ICiphernodeRegistry
    function publishCommittee(
        uint256 e3Id,
        bytes calldata,
        bytes calldata publicKey
    ) external {
        // only to be published by the filter
        require(address(registryFilters[e3Id]) == msg.sender, OnlyFilter());

        publicKeyHashes[e3Id] = keccak256(publicKey);
        emit CommitteePublished(e3Id, publicKey);
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
    function getFilter(uint256 e3Id) public view returns (address filter) {
        return address(registryFilters[e3Id]);
    }

    /// @inheritdoc ICiphernodeRegistry
    function getCommittee(
        uint256 e3Id
    ) public view returns (IRegistryFilter.Committee memory committee) {
        committee = registryFilters[e3Id].getCommittee(e3Id);
        require(committee.nodes.length > 0, CommitteeNotPublished());
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
