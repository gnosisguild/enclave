// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICommitteeCoordinator } from "../interfaces/ICommitteeCoordinator.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract NaiveCommitteeCoordinator is
    ICommitteeCoordinator,
    OwnableUpgradeable
{
    struct Committee {
        address[] nodes;
        uint32[2] threshold;
        bytes publicKey;
    }

    struct Node {
        bool eligible;
        // Number of duties the node has not yet completed.
        // Incremented each time a duty is added, decremented each time a duty is completed.
        uint256 outstandingDuties;
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
    error NoPublicKeyPublished();
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
        transferOwnership(_owner);
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
        Committee storage committee = committees[e3Id];
        require(committee.threshold.length == 0, CommitteeAlreadyExists());
        committee.threshold = threshold;
        success = true;
    }

    function publishCommittee(
        uint256 e3Id,
        address[] memory _nodes,
        bytes memory publicKey
    ) external onlyOwner {
        Committee storage committee = committees[e3Id];
        require(
            keccak256(committee.publicKey) == keccak256(hex""),
            CommitteeAlreadyPublished()
        );
        committee.nodes = _nodes;
        committee.publicKey = publicKey;
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

    function committeePublicKey(
        uint256 e3Id
    ) external view returns (bytes memory publicKey) {
        publicKey = committees[e3Id].publicKey;
        require(publicKey.length > 0, NoPublicKeyPublished());
    }

    /*
     * This naive registry does make store and make nodes available on chain
     * other more sophisticated registry may keep these on an off-chain tree
     * and store the root
     *
     * The idea is to not include this speficics on the main interfaces for
     * enclave for now
     */
    function getCommittee(
        uint256 e3Id
    ) external view returns (Committee memory committee) {
        committee = committees[e3Id];
        require(committees[e3Id].threshold.length > 0, CommitteeDoesNotExist());
    }
}
