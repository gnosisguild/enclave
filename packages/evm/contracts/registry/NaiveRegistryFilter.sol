// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IRegistryFilter } from "../interfaces/IRegistryFilter.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract NaiveRegistryFilter is IRegistryFilter, OwnableUpgradeable {
    struct Committee {
        address[] nodes;
        uint32[2] threshold;
        bytes publicKey;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    address public registry;

    mapping(uint256 e3 => Committee committee) public committees;

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
        address[] memory nodes,
        bytes memory publicKey
    ) external onlyOwner {
        Committee storage committee = committees[e3Id];
        require(
            keccak256(committee.publicKey) == keccak256(hex""),
            CommitteeAlreadyPublished()
        );
        committee.nodes = nodes;
        committee.publicKey = publicKey;
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
    ) external view returns (Committee memory) {
        return committees[e3Id];
    }
}
