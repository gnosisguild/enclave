// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

interface ICyphernodeRegistry {
    /// @notice This event MUST be emitted when a node is added to the registry.
    /// @param nodeId ID of the node.
    /// @param node Address of the node.
    event NodeAdded(uint256 indexed nodeId, address indexed node);

    /// @notice This event MUST be emitted when a node is removed from the registry.
    /// @param nodeId ID of the node.
    /// @param node Address of the node.
    event NodeRemoved(uint256 indexed nodeId, address indexed node);

    /// @notice This function should be called by the Enclave contract to select a node committee.
    /// @param e3Id ID of the E3 for which to select the committee.
    /// @param pools IDs of the pool of nodes from which to select the committee.
    /// @param threshold The M/N threshold for the committee.
    /// @return success True if committee selection was successfully initiated.
    function selectCommittee(
        uint256 e3Id,
        address[] memory pools,
        uint32[2] calldata threshold
    ) external returns (bool success);

    /// @notice This function should be called by the Enclave contract to get the public key of a committee.
    /// @dev This function MUST revert if no committee has been requested for the given E3.
    /// @dev This function MUST revert if the committee has not yet published a public key.
    /// @param e3Id ID of the E3 for which to get the committee public key.
    /// @return publicKey The public key of the committee.
    function getCommitteePublicKey(uint256 e3Id) external view returns (bytes memory publicKey);
}
