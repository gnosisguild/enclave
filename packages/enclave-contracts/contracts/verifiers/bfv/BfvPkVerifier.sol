// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity 0.8.28;

import { IPkVerifier } from "../../interfaces/IPkVerifier.sol";
import { ICircuitVerifier } from "../../interfaces/ICircuitVerifier.sol";
import { CommitteeHashLib } from "../../lib/CommitteeHashLib.sol";

/**
 * @title BfvPkVerifier
 * @notice Verifies the DkgAggregator (EVM) proof produced by the recursive
 *         aggregation pipeline (node folds + C5/pk_aggregation verified
 *         internally). Binds the proof to a caller-supplied `pkCommitment`
 *         and on-chain committee hash.
 * @dev Used when the Enclave is configured with encryptionSchemeId
 *      keccak256("fhe.rs:BFV"). The aggregator circuit's last public input is
 *      the hash-based aggregated PK commitment.
 */
contract BfvPkVerifier is IPkVerifier {
    /// @dev Must match `lib::configs::default::H` (micro committee size).
    uint256 internal constant H = 3;

    /// @dev `publicInputs` index for `committee_hash_hi` (after `party_ids`).
    uint256 internal constant COMMITTEE_HASH_HI_IDX = 2 + H;

    /// @dev `publicInputs` index for `committee_hash_lo` (after `party_ids`).
    uint256 internal constant COMMITTEE_HASH_LO_IDX = 3 + H;

    /// @dev `7` pub params + `8` return fields for micro `H = 3` (`dkg_aggregator`).
    uint256 internal constant EXPECTED_PUBLIC_INPUTS_LEN = 15;

    /// @dev Index of `pkCommitment` (last return field).
    uint256 internal constant PK_COMMITMENT_IDX =
        EXPECTED_PUBLIC_INPUTS_LEN - 1;

    /// @notice Underlying Honk verifier for the DkgAggregator circuit.
    ICircuitVerifier public immutable circuitVerifier;

    /// @notice Expected recursive VK hash for the nodes_fold sub-circuit (`publicInputs[0]`).
    bytes32 public immutable expectedNodesFoldKeyHash;

    /// @notice Expected recursive VK hash for the C5/pk_aggregation sub-circuit (`publicInputs[1]`).
    bytes32 public immutable expectedC5KeyHash;

    constructor(
        address _circuitVerifier,
        bytes32 _expectedNodesFoldKeyHash,
        bytes32 _expectedC5KeyHash
    ) {
        circuitVerifier = ICircuitVerifier(_circuitVerifier);
        expectedNodesFoldKeyHash = _expectedNodesFoldKeyHash;
        expectedC5KeyHash = _expectedC5KeyHash;
    }

    /// @inheritdoc IPkVerifier
    function verify(
        bytes32 pkCommitment,
        bytes32 committeeHash,
        bytes calldata proof
    ) external view override returns (bool) {
        (bytes memory rawProof, bytes32[] memory publicInputs) = abi.decode(
            proof,
            (bytes, bytes32[])
        );

        if (publicInputs.length != EXPECTED_PUBLIC_INPUTS_LEN) {
            return false;
        }
        if (publicInputs[0] != expectedNodesFoldKeyHash) {
            return false;
        }
        if (publicInputs[1] != expectedC5KeyHash) {
            return false;
        }
        if (
            publicInputs[COMMITTEE_HASH_HI_IDX] !=
            CommitteeHashLib.hi(committeeHash)
        ) {
            return false;
        }
        if (
            publicInputs[COMMITTEE_HASH_LO_IDX] !=
            CommitteeHashLib.lo(committeeHash)
        ) {
            return false;
        }
        if (publicInputs[PK_COMMITMENT_IDX] != pkCommitment) {
            return false;
        }
        return circuitVerifier.verify(rawProof, publicInputs);
    }
}
