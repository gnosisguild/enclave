// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

interface INodePool {
    /// @notice This function MUST return the Merkle root of this contract's curated pool of nodes.
    function merkleRoot() external returns (bytes32 merkleRoot);
}
