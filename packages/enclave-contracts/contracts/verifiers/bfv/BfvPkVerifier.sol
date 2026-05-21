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
 *      the hash-based aggregated PK commitment. Constructor `h` must match the
 *      compiled DkgAggregator honest-set size (`lib::configs::default::H`).
 */
contract BfvPkVerifier is IPkVerifier {
    /// @dev Debug-mode errors that pinpoint which check in `verify` failed.
    /// These replace the previous silent `return false` so callers (e.g. the
    /// registry's `require(verify(...), InvalidDkgProof())`) surface the
    /// specific failure selector instead of the generic `InvalidDkgProof`.
    error BadPublicInputsLen(uint256 actual, uint256 expected);
    error BadNodesFoldKeyHash(bytes32 actual, bytes32 expected);
    error BadC5KeyHash(bytes32 actual, bytes32 expected);
    error BadCommitteeHashHi(bytes32 actual, bytes32 expected);
    error BadCommitteeHashLo(bytes32 actual, bytes32 expected);
    error BadPkCommitment(bytes32 actual, bytes32 expected);

    /// @dev `dkg_aggregator` return field count: `1 + H + H + 1` (key hash + two `H` arrays + pk commitment).
    uint256 internal constant DKG_RETURN_TAIL_LEN = 2;

    /// @notice Honest-set size `H` (`party_ids` length); must match the compiled DkgAggregator circuit.
    uint256 public immutable h;

    /// @dev `publicInputs` index for `committee_hash_hi` (after `party_ids`).
    uint256 internal immutable committeeHashHiIdx;

    /// @dev `publicInputs` index for `committee_hash_lo`.
    uint256 internal immutable committeeHashLoIdx;

    /// @dev `2 + H + 2 + (2*H + DKG_RETURN_TAIL_LEN)` for `dkg_aggregator` EVM public inputs.
    uint256 internal immutable expectedPublicInputsLen;

    /// @dev Index of `pkCommitment` (last return field).
    uint256 internal immutable pkCommitmentIdx;

    /// @notice Underlying Honk verifier for the DkgAggregator circuit.
    ICircuitVerifier public immutable circuitVerifier;

    /// @notice Expected recursive VK hash for the nodes_fold sub-circuit (`publicInputs[0]`).
    bytes32 public immutable expectedNodesFoldKeyHash;

    /// @notice Expected recursive VK hash for the C5/pk_aggregation sub-circuit (`publicInputs[1]`).
    bytes32 public immutable expectedC5KeyHash;

    constructor(
        address _circuitVerifier,
        bytes32 _expectedNodesFoldKeyHash,
        bytes32 _expectedC5KeyHash,
        uint256 _h
    ) {
        require(_h > 0, "BfvPkVerifier: h=0");
        h = _h;
        committeeHashHiIdx = 2 + _h;
        committeeHashLoIdx = 3 + _h;
        expectedPublicInputsLen = (3 * _h) + 6;
        pkCommitmentIdx = expectedPublicInputsLen - 1;

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

        if (publicInputs.length != expectedPublicInputsLen) {
            revert BadPublicInputsLen(
                publicInputs.length,
                expectedPublicInputsLen
            );
        }
        if (publicInputs[0] != expectedNodesFoldKeyHash) {
            revert BadNodesFoldKeyHash(
                publicInputs[0],
                expectedNodesFoldKeyHash
            );
        }
        if (publicInputs[1] != expectedC5KeyHash) {
            revert BadC5KeyHash(publicInputs[1], expectedC5KeyHash);
        }
        bytes32 expectedHi = CommitteeHashLib.hi(committeeHash);
        if (publicInputs[committeeHashHiIdx] != expectedHi) {
            revert BadCommitteeHashHi(
                publicInputs[committeeHashHiIdx],
                expectedHi
            );
        }
        bytes32 expectedLo = CommitteeHashLib.lo(committeeHash);
        if (publicInputs[committeeHashLoIdx] != expectedLo) {
            revert BadCommitteeHashLo(
                publicInputs[committeeHashLoIdx],
                expectedLo
            );
        }
        if (publicInputs[pkCommitmentIdx] != pkCommitment) {
            revert BadPkCommitment(publicInputs[pkCommitmentIdx], pkCommitment);
        }
        return circuitVerifier.verify(rawProof, publicInputs);
    }
}
