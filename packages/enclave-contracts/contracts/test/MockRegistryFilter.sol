// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IRegistryFilter } from "../interfaces/IRegistryFilter.sol";
import {
    OwnableUpgradeable
} from "@oz-upgradeable/access/OwnableUpgradeable.sol";

interface IRegistry {
    function publishCommittee(
        uint256 e3Id,
        address[] calldata ciphernodes,
        bytes calldata publicKey
    ) external;
}

contract MockNaiveRegistryFilter is IRegistryFilter, OwnableUpgradeable {
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    address public registry;

    mapping(uint256 e3 => IRegistryFilter.Committee committee)
        public committees;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    error CommitteeAlreadyExists();
    error CommitteeAlreadyPublished();
    error CommitteeDoesNotExist();
    error CommitteeNotPublished();
    error OnlyRegistry();

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                     Modifiers                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    modifier onlyRegistry() {
        require(msg.sender == registry, OnlyRegistry());
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

    function initialize(address _owner, address _registry) public initializer {
        __Ownable_init(msg.sender);
        setRegistry(_registry);
        if (_owner != owner()) transferOwnership(_owner);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Core Entrypoints                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function requestCommittee(
        uint256 e3Id,
        uint32[2] calldata threshold
    ) external onlyRegistry returns (bool success) {
        IRegistryFilter.Committee storage committee = committees[e3Id];
        require(committee.threshold.length == 0, CommitteeAlreadyExists());
        committee.threshold = threshold;
        success = true;
    }

    function publishCommittee(
        uint256 e3Id,
        address[] memory nodes,
        bytes memory publicKey
    ) external onlyOwner {
        IRegistryFilter.Committee storage committee = committees[e3Id];
        require(committee.publicKey == bytes32(0), CommitteeAlreadyPublished());
        committee.nodes = nodes;
        committee.publicKey = keccak256(publicKey);
        IRegistry(registry).publishCommittee(e3Id, nodes, publicKey);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Set Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function setRegistry(address _registry) public onlyOwner {
        registry = _registry;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function getCommittee(
        uint256 e3Id
    ) external view returns (IRegistryFilter.Committee memory) {
        IRegistryFilter.Committee memory committee = committees[e3Id];
        require(committee.publicKey != bytes32(0), CommitteeNotPublished());
        return committee;
    }
}
