// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

interface ICypherNodeRegistry {
    /// @notice This function should be called by the Enclave contract to select a node committee.
    /// @param poolId ID of the pool of nodes from which to select the committee.
    /// @param threshold The M/N threshold for the committee.
    /// @return committeeId ID of the selected committee.
    function selectCommittee(uint256 poolId, uint32[2] calldata threshold) external returns (bytes32 committeeId);
}
