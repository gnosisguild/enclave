// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICyphernodeRegistry } from "../interfaces/ICyphernodeRegistry.sol";
import { ICyphernodeState } from "../interfaces/ICyphernodeState.sol";
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
    address public state;

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

    constructor(address _owner, address _enclave, address _state) {
        initialize(_owner, _enclave, _state);
    }

    function initialize(
        address _owner,
        address _enclave,
        address _state
    ) public initializer {
        __Ownable_init(msg.sender);
        setEnclave(_enclave);
        setEnclave(_state);
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

    function publishCommittee(
        uint256 e3Id,
        bytes calldata proof,
        bytes calldata publicKey
    ) external {
        // only to be published by the filter
        require(address(requests[e3Id]) == msg.sender, CommitteeDoesNotExist());

        ICyphernodeState(state).verifyCommittee(e3Id, proof, publicKey);

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

    function setState(address _state) public onlyOwner {
        state = _state;
    }

    function isCyphernodeEligible(address node) external view returns (bool) {
        return ICyphernodeState(state).isCyphernodeEligible(node);
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
