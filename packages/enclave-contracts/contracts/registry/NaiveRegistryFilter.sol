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

/**
 * @title NaiveRegistryFilter
 * @notice Simple registry filter implementation for committee selection
 * @dev Allows owner-controlled committee publication for E3 computations
 */
contract NaiveRegistryFilter is IRegistryFilter, OwnableUpgradeable {
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Address of the ciphernode registry contract
    address public registry;

    /// @notice Maps E3 ID to its committee data
    mapping(uint256 e3 => IRegistryFilter.Committee committee)
        public committees;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Committee already exists for this E3
    error CommitteeAlreadyExists();

    /// @notice Committee has already been published for this E3
    error CommitteeAlreadyPublished();

    /// @notice Committee does not exist for this E3
    error CommitteeDoesNotExist();

    /// @notice Committee has not been published yet
    error CommitteeNotPublished();

    /// @notice Ciphernode is not enabled in the registry
    /// @param ciphernode Address of the ciphernode
    error CiphernodeNotEnabled(address ciphernode);

    /// @notice Caller is not the registry contract
    error OnlyRegistry();

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                     Modifiers                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @dev Restricts function access to only the registry contract
    modifier onlyRegistry() {
        require(msg.sender == registry, OnlyRegistry());
        _;
    }

    /// @dev Restricts function access to owner or eligible ciphernode
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

    /// @notice Constructor that initializes the filter with owner and registry
    /// @param _owner Address that will own the contract
    /// @param _registry Address of the ciphernode registry
    constructor(address _owner, address _registry) {
        initialize(_owner, _registry);
    }

    /// @notice Initializes the filter contract
    /// @dev Can only be called once due to initializer modifier
    /// @param _owner Address that will own the contract
    /// @param _registry Address of the ciphernode registry
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

    /// @inheritdoc IRegistryFilter
    function requestCommittee(
        uint256 e3Id,
        uint32[2] calldata threshold
    ) external onlyRegistry returns (bool success) {
        require(committees[e3Id].threshold[1] == 0, CommitteeAlreadyExists());
        committees[e3Id].threshold = threshold;
        success = true;
    }

    /// @notice Publishes a committee for an E3 computation
    /// @dev Only callable by owner. Stores committee data and notifies the registry
    /// @param e3Id ID of the E3 computation
    /// @param nodes Array of ciphernode addresses selected for the committee
    /// @param publicKey Aggregated public key of the committee
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

    /// @notice Sets the registry contract address
    /// @dev Only callable by owner
    /// @param _registry Address of the ciphernode registry contract
    function setRegistry(address _registry) public onlyOwner {
        registry = _registry;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @inheritdoc IRegistryFilter
    function getCommittee(
        uint256 e3Id
    ) external view returns (IRegistryFilter.Committee memory) {
        IRegistryFilter.Committee memory committee = committees[e3Id];
        require(committee.publicKey != bytes32(0), CommitteeNotPublished());
        return committee;
    }
}
