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
 *         internally) and binds it to the full on-chain call context.
 * @dev Used when the Interfold is configured with encryptionSchemeId
 *      keccak256("fhe.rs:BFV"). Constructor `h` must match the compiled
 *      DkgAggregator honest-set size (`lib::configs::default::H`).
 *
 *      Expected `publicInputs` layout for DkgAggregator EVM outputs:
 *        [0]                = expectedNodesFoldKeyHash  (complete VK-tree anchor)
 *        [1]                = expectedC5KeyHash         (VK anchor)
 *        [2 .. 2+H)         = party_ids                 (H slots)
 *        [2+H]              = committee_hash_hi
 *        [3+H]              = committee_hash_lo
 *        [4+H]              = aggregate recursive VK key hash
 *        [5+H .. 5+2H)      = aggregate SK commitments
 *        [5+2H]             = pk_commitment
 *        [6+2H .. end)      = outgoing PRF commitments [H][N]
 *        Total: 6 + 2*H + H*N, with N = sortedNodes.length.
 *
 *      The two VK-hash slots are checked against contract immutables set at
 *      construction; this anchors the recursive aggregation trust and
 *      prevents a malicious aggregator from substituting a forged sub-VK.
 *
 *      NOTE — domain binding relaxation: wrapper-level chainId/deployment/e3Id
 *      binding requires a dedicated circuit public input. The current circuits
 *      do not expose such a slot. Full cryptographic enforcement tracked as
 *      future work. The caller-supplied `e3Id`, `committeeRoot`, and
 *      `sortedNodes` are preserved in the interface for forward compatibility.
 */
contract BfvPkVerifier is IPkVerifier {
    error InvalidCircuitVerifier(address verifier);
    error InvalidVerificationKeyHash();
    error NonCanonicalPublicInput(uint256 index);

    /// @notice Honest-set size `H` (`party_ids` length); must match the compiled DkgAggregator circuit.
    uint256 public immutable override h;

    /// @dev `publicInputs` index for `committee_hash_hi` (after VK anchors and `party_ids`).
    uint256 internal immutable committeeHashHiIdx;

    /// @dev `publicInputs` index for `committee_hash_lo`.
    uint256 internal immutable committeeHashLoIdx;

    /// @dev Public-input prefix length, excluding the dynamically sized H*N key grid.
    uint256 internal immutable expectedPublicInputsLen;

    /// @dev Index of `pkCommitment` before the outgoing-key grid.
    uint256 internal immutable pkCommitmentIdx;

    /// @notice Underlying Honk verifier for the DkgAggregator circuit.
    ICircuitVerifier public immutable circuitVerifier;

    /// @notice SAFE commitment to every VK in the nodes-fold proof tree.
    ///         Expected at `publicInputs[0]`. The builder writes
    ///         `nodes_fold.vk_tree_hash` from the complete artifact pair.
    bytes32 public immutable expectedNodesFoldKeyHash;

    /// @notice Barretenberg hash of the C5 non-ZK recursive VK.
    ///         Expected at `publicInputs[1]` from `pk_aggregation.vk_hash`.
    bytes32 public immutable expectedC5KeyHash;

    constructor(
        address _circuitVerifier,
        bytes32 _expectedNodesFoldKeyHash,
        bytes32 _expectedC5KeyHash,
        uint256 _h
    ) {
        require(_h > 0, "BfvPkVerifier: h=0");
        if (_circuitVerifier.code.length == 0) {
            revert InvalidCircuitVerifier(_circuitVerifier);
        }
        if (
            _expectedNodesFoldKeyHash == bytes32(0) ||
            _expectedC5KeyHash == bytes32(0)
        ) revert InvalidVerificationKeyHash();
        h = _h;
        committeeHashHiIdx = 2 + _h;
        committeeHashLoIdx = 3 + _h;
        // The final H*N PRF grid is sized from the canonical full committee at verification.
        expectedPublicInputsLen = (2 * _h) + 6;
        pkCommitmentIdx = 5 + 2 * _h;

        circuitVerifier = ICircuitVerifier(_circuitVerifier);
        expectedNodesFoldKeyHash = _expectedNodesFoldKeyHash;
        expectedC5KeyHash = _expectedC5KeyHash;
    }

    /// @inheritdoc IPkVerifier
    function verify(
        uint256 e3Id,
        uint256 committeeRoot,
        address[] calldata sortedNodes,
        bytes32 pkCommitment,
        bytes32 committeeHash,
        bytes calldata proof
    ) external view override returns (bool) {
        (bytes memory rawProof, bytes32[] memory publicInputs) = abi.decode(
            proof,
            (bytes, bytes32[])
        );

        if (
            publicInputs.length !=
            expectedPublicInputsLen + h * sortedNodes.length
        ) {
            revert InvalidPublicInputsLength();
        }
        for (uint256 i; i < publicInputs.length; ++i) {
            if (
                uint256(publicInputs[i]) >=
                21888242871839275222246405745257275088548364400416034343698204186575808495617
            ) {
                revert NonCanonicalPublicInput(i);
            }
        }

        // Anchor recursive-aggregation trust to immutable VK hashes.
        if (publicInputs[0] != expectedNodesFoldKeyHash) {
            revert VkHashMismatch();
        }
        if (publicInputs[1] != expectedC5KeyHash) {
            revert VkHashMismatch();
        }

        // Bind to the on-chain committee hash (hi/lo split per Noir field convention).
        if (
            publicInputs[committeeHashHiIdx] !=
            CommitteeHashLib.hi(committeeHash)
        ) {
            revert DomainBindingMismatch();
        }
        if (
            publicInputs[committeeHashLoIdx] !=
            CommitteeHashLib.lo(committeeHash)
        ) {
            revert DomainBindingMismatch();
        }

        // Aggregated PK commitment is the last slot.
        if (publicInputs[pkCommitmentIdx] != pkCommitment) {
            revert PkCommitmentMismatch();
        }

        // Suppress unused-variable warnings for forward-compatibility params.
        // These will be used for circuit-level domain binding in a future circuit update.
        e3Id;
        committeeRoot;
        sortedNodes;

        // Bubble up as a revert instead of a silent `false`.
        if (!circuitVerifier.verify(rawProof, publicInputs)) {
            revert InvalidProof();
        }
        return true;
    }
}
