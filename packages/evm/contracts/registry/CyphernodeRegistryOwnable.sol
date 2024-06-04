// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICyphernodeRegistry } from "../interfaces/ICyphernodeRegistry.sol";
import { OwnableUpgradeable } from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract CyphernodeRegistryOwnable is ICyphernodeRegistry, OwnableUpgradeable {
    struct Committee {
        address[] nodes;
        uint32[2] threshold;
        address[] pools;
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

    mapping(uint256 e3 => Committee committee) public committees;
    mapping(address nodeId => Node node) public nodes;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Initialization                       //
    //                                                        //
    ////////////////////////////////////////////////////////////

    constructor(address _owner) {
        initialize(_owner);
    }

    function initialize(address _owner) public initializer {
        __Ownable_init(_owner);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Core Entrypoints                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function selectCommittee(
        uint256 e3Id,
        address[] memory pools,
        uint32[2] calldata threshold
    ) external pure override returns (bool success) {}

    function getCommitteePublicKey(uint256 e3Id) external pure override returns (bytes memory) {}

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Set Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////
}
