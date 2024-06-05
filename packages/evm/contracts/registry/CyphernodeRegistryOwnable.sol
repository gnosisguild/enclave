// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICyphernodeRegistry } from "../interfaces/ICyphernodeRegistry.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract CyphernodeRegistryOwnable is ICyphernodeRegistry, OwnableUpgradeable {
    struct Committee {
        address[] nodes;
        uint32[2] threshold;
        address[] pools;
        bytes32[] merkleRoots;
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

    address public enclave;

    mapping(uint256 e3 => Committee committee) public committees;
    mapping(address nodeId => Node node) public nodes;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

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

    function selectCommittee(
        uint256 e3Id,
        address[] memory pools,
        uint32[2] calldata threshold
    ) external onlyEnclave returns (bool success) {
        Committee storage committee = committees[e3Id];
        require(committee.threshold.length == 0, CommitteeAlreadyExists());
        committee.threshold = threshold;
        committee.pools = pools;
        success = true;

        emit CommitteeRequested(e3Id, pools, threshold);
    }

    function publishCommittee(
        uint256 e3Id,
        address[] memory _nodes,
        bytes32[] memory merkleRoots,
        bytes memory publicKey
    ) external onlyOwner {
        Committee storage committee = committees[e3Id];
        require(keccak256(committee.publicKey) == keccak256(hex""), CommitteeAlreadyPublished());
        committee.nodes = _nodes;
        committee.merkleRoots = merkleRoots;
        committee.publicKey = publicKey;

        emit CommitteeSelected(e3Id, _nodes, merkleRoots, publicKey);
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

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function getCommitteePublicKey(uint256 e3Id) external view returns (bytes memory publicKey) {
        publicKey = committees[e3Id].publicKey;
        require(publicKey.length > 0, NoPublicKeyPublished());
    }

    function getCommittee(uint256 e3Id) external view returns (Committee memory committee) {
        committee = committees[e3Id];
        require(committees[e3Id].threshold.length > 0, CommitteeDoesNotExist());
    }
}
