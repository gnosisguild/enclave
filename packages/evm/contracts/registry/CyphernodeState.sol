// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICyphernodeState } from "../interfaces/ICyphernodeState.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract CyphernodeRegistryOwnable is ICyphernodeState, OwnableUpgradeable {
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    mapping(address cyphernode => bool isEnabled) public isEnabled;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    error CyphernodeNotEligible(address node);

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Initialization                       //
    //                                                        //
    ////////////////////////////////////////////////////////////

    constructor(address _owner) {
        initialize(_owner);
    }

    function initialize(address _owner) public initializer {
        __Ownable_init(msg.sender);
        transferOwnership(_owner);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Core Entrypoints                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function verifyCommittee(
        uint256,
        bytes calldata proof,
        bytes calldata
    ) external view {
        address[] memory cyphernodes = abi.decode(proof, (address[]));

        for (uint256 i = 0; i < cyphernodes.length; i++) {
            require(
                isEnabled[cyphernodes[i]] == true,
                CyphernodeNotEligible(cyphernodes[i])
            );
        }
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Set Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////
    function addCyphernode(address node) external onlyOwner {
        isEnabled[node] = true;
        emit CyphernodeAdded(node);
    }

    function removeCyphernode(address node) external onlyOwner {
        isEnabled[node] = false;
        emit CyphernodeRemoved(node);
    }

    function isCyphernodeEligible(address node) external view returns (bool) {
        return isEnabled[node];
    }
}
