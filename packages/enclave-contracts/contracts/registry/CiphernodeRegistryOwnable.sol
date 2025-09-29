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

contract CiphernodeRegistryOwnable is ICiphernodeRegistry, OwnableUpgradeable {
    using InternalLeanIMT for LeanIMTData;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Events                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    event BondingRegistrySet(address indexed bondingRegistry);

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    address public enclave;
    address public bondingRegistry;
    uint256 public numCiphernodes;
    LeanIMTData public ciphernodes;

    mapping(uint256 e3Id => IRegistryFilter filter) public registryFilters;
    mapping(uint256 e3Id => uint256 root) public roots;
    mapping(uint256 e3Id => bytes32 publicKeyHash) public publicKeyHashes;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    error CommitteeAlreadyRequested();
    error CommitteeAlreadyPublished();
    error OnlyFilter();
    error CommitteeNotPublished();
    error CiphernodeNotEnabled(address node);
    error OnlyEnclave();
    error OnlyBondingRegistry();
    error NotOwnerOrBondingRegistry();
    error NodeNotBonded(address node);
    error ZeroAddress();
    error BondingRegistryNotSet();
    error Unauthorized();

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                     Modifiers                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    modifier onlyEnclave() {
        require(msg.sender == enclave, OnlyEnclave());
        _;
    }

    modifier onlyBondingRegistry() {
        require(msg.sender == bondingRegistry, OnlyBondingRegistry());
        _;
    }

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

    constructor(address _owner, address _enclave) {
        initialize(_owner, _enclave);
    }

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

    function setEnclave(address _enclave) public onlyOwner {
        require(_enclave != address(0), ZeroAddress());
        enclave = _enclave;
        emit EnclaveSet(_enclave);
    }

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

    function committeePublicKey(
        uint256 e3Id
    ) external view returns (bytes32 publicKeyHash) {
        publicKeyHash = publicKeyHashes[e3Id];
        require(publicKeyHash != bytes32(0), CommitteeNotPublished());
    }

    function isCiphernodeEligible(address node) external view returns (bool) {
        if (!isEnabled(node)) return false;

        require(bondingRegistry != address(0), BondingRegistryNotSet());
        return IBondingRegistry(bondingRegistry).isActive(node);
    }

    function isEnabled(address node) public view returns (bool) {
        return ciphernodes._has(uint160(node));
    }

    function root() public view returns (uint256) {
        return (ciphernodes._root());
    }

    function rootAt(uint256 e3Id) public view returns (uint256) {
        return roots[e3Id];
    }

    function getFilter(uint256 e3Id) public view returns (address filter) {
        return address(registryFilters[e3Id]);
    }

    function getCommittee(
        uint256 e3Id
    ) public view returns (IRegistryFilter.Committee memory) {
        return registryFilters[e3Id].getCommittee(e3Id);
    }

    function treeSize() public view returns (uint256) {
        return ciphernodes.size;
    }

    function getBondingRegistry() external view returns (address) {
        return bondingRegistry;
    }
}
