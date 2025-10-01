// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IRegistryFilter } from "../interfaces/IRegistryFilter.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract NaiveRegistryFilter is IRegistryFilter, OwnableUpgradeable {
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
    error CiphernodeNotEnabled(address ciphernode);
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

    modifier onlyOwnerOrCiphernode() {
        require(
            msg.sender == owner() ||
                ICiphernodeRegistry(registry).isCiphernodeEligible(msg.sender),
            CiphernodeNotEnabled(msg.sender)
        );
        _;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Initialization                       //
    //                                                        //
    ////////////////////////////////////////////////////////////

    constructor(address _owner, address _registry) {
        initialize(_owner, _registry);
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
        require(committees[e3Id].threshold[1] == 0, CommitteeAlreadyExists());
        committees[e3Id].threshold = threshold;
        success = true;
    }

    function publishCommittee(
        uint256 e3Id,
        address[] calldata nodes,
        bytes calldata publicKey
    ) external onlyOwner {
        IRegistryFilter.Committee storage committee = committees[e3Id];
        require(committee.publicKey == bytes32(0), CommitteeAlreadyPublished());
        committee.nodes = nodes;
        committee.publicKey = keccak256(publicKey);
        ICiphernodeRegistry(registry).publishCommittee(
            e3Id,
            abi.encode(nodes),
            publicKey
        );
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
