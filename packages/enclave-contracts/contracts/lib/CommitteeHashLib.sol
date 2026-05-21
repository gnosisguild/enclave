// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity 0.8.28;

/**
 * @title CommitteeHashLib
 * @notice Canonical `keccak256(abi.encodePacked(topNodes))` binding for aggregator proofs.
 * @dev Must match `e3_utils::committee_hash` (hi/lo split into two 128-bit limbs).
 */
library CommitteeHashLib {
    uint256 private constant _LO_MASK = (uint256(1) << 128) - 1;

    /// @notice `keccak256(abi.encodePacked(nodes))` for the ordered on-chain committee.
    /// @dev Callers pass `storage` arrays via implicit copy to this `memory` parameter.
    function hash(address[] memory nodes) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(nodes));
    }

    /// @notice High 128 bits of a committee hash (Noir public input `committee_hash_hi`).
    function hi(bytes32 committeeHash) internal pure returns (bytes32) {
        return bytes32(uint256(committeeHash) >> 128);
    }

    /// @notice Low 128 bits of a committee hash (Noir public input `committee_hash_lo`).
    function lo(bytes32 committeeHash) internal pure returns (bytes32) {
        return bytes32(uint256(committeeHash) & _LO_MASK);
    }
}
