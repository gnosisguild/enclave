// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICyphernodeRegistry } from "../interfaces/ICyphernodeRegistry.sol";
import { ICommitteeCoordinator } from "../interfaces/ICommitteeCoordinator.sol";
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

    mapping(address filter => bool enabled) public filters;

    mapping(uint256 e3 => ICommitteeCoordinator coordinator) public requests;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    error CommitteeAlreadyRequested();
    error CommitteeAlreadyExists();
    error CommitteeAlreadyPublished();
    error CommitteeDoesNotExist();
    error NoPublicKeyPublished();
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
            requests[e3Id] == ICommitteeCoordinator(address(0)),
            CommitteeAlreadyRequested()
        );
        requests[e3Id] = ICommitteeCoordinator(filter);

        ICommitteeCoordinator(filter).requestCommittee(e3Id, threshold);

        emit CommitteeRequested(e3Id, filter, threshold);
        success = true;
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

    function addFilter(address filter) external {
        filters[filter] = true;
        emit FilterAdded(filter);
    }

    function removeFilter(address filter) external onlyOwner {
        filters[filter] = false;
        emit FilterRemoved(filter);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function committeePublicKey(
        uint256 e3Id
    ) external view returns (bytes memory publicKey) {
        require(requests[e3Id] != ICommitteeCoordinator(address(0)));

        publicKey = requests[e3Id].committeePublicKey(e3Id);
        require(publicKey.length > 0, NoPublicKeyPublished());
    }
}
