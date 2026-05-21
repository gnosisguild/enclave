// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity 0.8.28;

import { IDecryptionVerifier } from "../../interfaces/IDecryptionVerifier.sol";
import { ICircuitVerifier } from "../../interfaces/ICircuitVerifier.sol";
import { CommitteeHashLib } from "../../lib/CommitteeHashLib.sol";

/**
 * @title BfvDecryptionVerifier
 * @notice Verifies the DecryptionAggregator (EVM) proof produced by the
 *         recursive aggregation pipeline (C6 folds + C7/decrypted_shares
 *         verified internally). Binds the proof to the claimed
 *         `plaintextOutputHash` and on-chain committee hash.
 * @dev Used when the Enclave is configured with encryptionSchemeId
 *      keccak256("fhe.rs:BFV"). The plaintext is exposed as the last
 *      `MESSAGE_COEFFS_COUNT` public inputs, matching
 *      `MAX_MSG_NON_ZERO_COEFFS` in the decryption_aggregator circuit.
 *      Constructor `threshold` must match the compiled circuit `T`
 *      (`lib::configs::default::T`). Committee hash limbs are always at
 *      indices 2 and 3; total public-input length is preset-dependent.
 */
contract BfvDecryptionVerifier is IDecryptionVerifier {
    /// @dev Debug-mode errors that pinpoint which check in `verify` failed.
    /// These replace the previous silent `return false` so callers (e.g. Enclave's
    /// `require(verify(...), InvalidDecryptionProof())`) surface the specific failure.
    error BadPublicInputsLen(uint256 actual, uint256 expected);
    error BadC6FoldKeyHash(bytes32 actual, bytes32 expected);
    error BadC7KeyHash(bytes32 actual, bytes32 expected);
    error BadCommitteeHashHi(bytes32 actual, bytes32 expected);
    error BadCommitteeHashLo(bytes32 actual, bytes32 expected);
    error BadPlaintextHash(bytes32 actual, bytes32 expected);

    /// @dev Message is always the last 100 public inputs (100 uint64 coeffs = 800 bytes plaintext).
    uint256 internal constant MESSAGE_COEFFS_COUNT = 100;

    /// @dev `decryption_aggregator` return tail: `1 + 3*(T+1) + MESSAGE_COEFFS_COUNT` fields.
    uint256 internal constant DEC_RETURN_PREFIX_LEN = 1;

    /// @dev `decryption_aggregator` return columns after the leading key hash (sk, esm, ct).
    uint256 internal constant DEC_RETURN_COLUMN_COUNT = 3;

    /// @dev `publicInputs` index for `committee_hash_hi` (after sub-circuit key hashes).
    uint256 internal constant COMMITTEE_HASH_HI_IDX = 2;

    /// @dev `publicInputs` index for `committee_hash_lo`.
    uint256 internal constant COMMITTEE_HASH_LO_IDX = 3;

    /// @notice BFV threshold `T`; must match the compiled DecryptionAggregator circuit.
    uint256 public immutable threshold;

    /// @dev `4 + DEC_RETURN_PREFIX_LEN + DEC_RETURN_COLUMN_COUNT*(T+1) + MESSAGE_COEFFS_COUNT`.
    uint256 internal immutable expectedPublicInputsLen;

    /// @notice Underlying Honk verifier for the DecryptionAggregator circuit.
    ICircuitVerifier public immutable circuitVerifier;

    /// @notice Expected recursive VK hash for the c6_fold sub-circuit (`publicInputs[0]`).
    bytes32 public immutable expectedC6FoldKeyHash;

    /// @notice Expected recursive VK hash for the C7/decrypted_shares_aggregation sub-circuit (`publicInputs[1]`).
    bytes32 public immutable expectedC7KeyHash;

    constructor(
        address _circuitVerifier,
        bytes32 _expectedC6FoldKeyHash,
        bytes32 _expectedC7KeyHash,
        uint256 _threshold
    ) {
        require(_threshold > 0, "BfvDecryptionVerifier: threshold=0");
        threshold = _threshold;
        expectedPublicInputsLen =
            4 +
            DEC_RETURN_PREFIX_LEN +
            (DEC_RETURN_COLUMN_COUNT * (_threshold + 1)) +
            MESSAGE_COEFFS_COUNT;

        circuitVerifier = ICircuitVerifier(_circuitVerifier);
        expectedC6FoldKeyHash = _expectedC6FoldKeyHash;
        expectedC7KeyHash = _expectedC7KeyHash;
    }

    /// @inheritdoc IDecryptionVerifier
    function verify(
        bytes32 plaintextOutputHash,
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
        if (publicInputs[0] != expectedC6FoldKeyHash) {
            revert BadC6FoldKeyHash(publicInputs[0], expectedC6FoldKeyHash);
        }
        if (publicInputs[1] != expectedC7KeyHash) {
            revert BadC7KeyHash(publicInputs[1], expectedC7KeyHash);
        }
        bytes32 expectedHi = CommitteeHashLib.hi(committeeHash);
        if (publicInputs[COMMITTEE_HASH_HI_IDX] != expectedHi) {
            revert BadCommitteeHashHi(
                publicInputs[COMMITTEE_HASH_HI_IDX],
                expectedHi
            );
        }
        bytes32 expectedLo = CommitteeHashLib.lo(committeeHash);
        if (publicInputs[COMMITTEE_HASH_LO_IDX] != expectedLo) {
            revert BadCommitteeHashLo(
                publicInputs[COMMITTEE_HASH_LO_IDX],
                expectedLo
            );
        }
        bytes32 actualPlaintextHash = _computePlaintextHash(publicInputs);
        if (actualPlaintextHash != plaintextOutputHash) {
            revert BadPlaintextHash(actualPlaintextHash, plaintextOutputHash);
        }
        return circuitVerifier.verify(rawProof, publicInputs);
    }

    function _computePlaintextHash(
        bytes32[] memory publicInputs
    ) internal view returns (bytes32) {
        uint256 offset = expectedPublicInputsLen - MESSAGE_COEFFS_COUNT;
        bytes memory plaintext = new bytes(MESSAGE_COEFFS_COUNT * 8);
        for (uint256 i = 0; i < MESSAGE_COEFFS_COUNT; i++) {
            uint64 coeff = uint64(uint256(publicInputs[offset + i]));
            for (uint256 j = 0; j < 8; j++) {
                plaintext[i * 8 + j] = bytes1(uint8(coeff >> (j * 8)));
            }
        }
        return keccak256(plaintext);
    }
}
