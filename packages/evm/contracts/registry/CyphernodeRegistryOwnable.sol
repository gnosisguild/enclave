// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICyphernodeRegistry } from "../interfaces/ICyphernodeRegistry.sol";
import { IRegistryFilter } from "../interfaces/IRegistryFilter.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract CyphernodeRegistryOwnable is ICyphernodeRegistry, OwnableUpgradeable {
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    address public enclave;

    mapping(address cyphernode => bool isEnabled) public isEnabled;

    mapping(uint256 e3Id => IRegistryFilter filter) public requests;
    mapping(uint256 e3Id => bytes publicKey) public publicKeys;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    error CommitteeAlreadyRequested();
    error CommitteeAlreadyPublished();
    error CommitteeDoesNotExist();
    error CommitteeNotPublished();
    error CyphernodeNotEnabled(address node);
    error OnlyEnclave();

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                     Modifiers                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    modifier onlyEnclave() {
        require(msg.sender == enclave, OnlyEnclave());
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
        __Ownable_init(msg.sender);
        setEnclave(_enclave);
        transferOwnership(_owner);
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
            requests[e3Id] == IRegistryFilter(address(0)),
            CommitteeAlreadyRequested()
        );
        requests[e3Id] = IRegistryFilter(filter);

        IRegistryFilter(filter).requestCommittee(e3Id, threshold);
        emit CommitteeRequested(e3Id, filter, threshold);
        success = true;
    }

    function publishCommittee(uint256 e3Id, bytes calldata publicKey) external {
        // only to be published by the filter
        require(address(requests[e3Id]) == msg.sender, CommitteeDoesNotExist());

        // for (uint256 i = 0; i < cyphernodes.length; i++) {
        //     require(
        //         isEnabled[cyphernodes[i]] == true,
        //         CyphernodeNotEnabled(cyphernodes[i])
        //     );
        // }

        publicKeys[e3Id] = publicKey;
        emit CommitteePublished(e3Id, publicKey);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Set Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function setEnclave(address _enclave) public onlyOwner {
        enclave = _enclave;
        emit EnclaveSet(_enclave);
    }

    function addCyphernode(address node) external onlyOwner {
        isEnabled[node] = true;
        emit CyphernodeAdded(node);
    }

    function removeCyphernode(address node) external onlyOwner {
        isEnabled[node] = false;
        emit CyphernodeRemoved(node);
    }

    function isCyphernodeEnabled(address node) external view returns (bool) {
        return isEnabled[node];
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function committeePublicKey(
        uint256 e3Id
    ) external view returns (bytes memory publicKey) {
        publicKey = publicKeys[e3Id];
        require(publicKey.length > 0, CommitteeNotPublished());
    }
}
