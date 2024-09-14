// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IRegistryFilter } from "../interfaces/IRegistryFilter.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {
    InternalLeanIMT,
    LeanIMTData,
    PoseidonT3
} from "@zk-kit/lean-imt.sol/InternalLeanIMT.sol";

contract CiphernodeRegistryOwnable is ICiphernodeRegistry, OwnableUpgradeable {
    using InternalLeanIMT for LeanIMTData;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    address public enclave;
    uint256 public numCiphernodes;
    LeanIMTData public ciphernodes;

    mapping(uint256 e3Id => IRegistryFilter filter) public requests;
    mapping(uint256 e3Id => uint256 root) public roots;
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
    error CiphernodeNotEnabled(address node);
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
        require(address(requests[e3Id]) == msg.sender, CommitteeDoesNotExist());

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

    function addCiphernode(address node) external onlyOwner {
        uint256 ciphernode = uint256(bytes32(bytes20(node)));
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
    ) external onlyOwner {
        uint256 ciphernode = uint256(bytes32(bytes20(node)));
        ciphernodes._remove(ciphernode, siblingNodes);
        uint256 index = ciphernodes._indexOf(ciphernode);
        numCiphernodes--;
        emit CiphernodeAdded(
            node,
            ciphernodes._indexOf(ciphernode),
            numCiphernodes,
            ciphernodes.size
        );
        emit CiphernodeRemoved(node, index, numCiphernodes, ciphernodes.size);
    }

    function isCiphernodeEligible(address node) external view returns (bool) {
        return isEnabled(node);
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

    function isEnabled(address node) public view returns (bool) {
        return ciphernodes._has(uint256(bytes32(bytes20(node))));
    }

    function root() public view returns (uint256) {
        return (ciphernodes._root());
    }

    function rootAt(uint256 e3Id) public view returns (uint256) {
        return roots[e3Id];
    }
}
