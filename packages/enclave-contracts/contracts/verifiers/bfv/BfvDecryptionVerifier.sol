// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IDecryptionVerifier } from "../../interfaces/IDecryptionVerifier.sol";
import { ICircuitVerifier } from "../../interfaces/ICircuitVerifier.sol";

/**
 * @title BfvDecryptionVerifier
 * @notice Verifies the DecryptionAggregator (EVM) proof produced by the
 *         recursive aggregation pipeline (C6 folds + C7/decrypted_shares
 *         verified internally). Binds the proof to the claimed
 *         `plaintextOutputHash`.
 * @dev Used when the Enclave is configured with encryptionSchemeId
 *      keccak256("fhe.rs:BFV"). The plaintext is exposed as the last
 *      `MESSAGE_COEFFS_COUNT` public inputs, matching
 *      `MAX_MSG_NON_ZERO_COEFFS` in the decryption_aggregator circuit.
 */
contract BfvDecryptionVerifier is IDecryptionVerifier {
    /// @dev Message is always the last 100 public inputs (100 uint64 coeffs = 800 bytes plaintext).
    uint256 constant MESSAGE_COEFFS_COUNT = 100;

    /// @notice Underlying Honk verifier for the DecryptionAggregator circuit.
    ICircuitVerifier public immutable circuitVerifier;

    constructor(address _circuitVerifier) {
        circuitVerifier = ICircuitVerifier(_circuitVerifier);
    }

    /// @inheritdoc IDecryptionVerifier
    function verify(
        bytes32 plaintextOutputHash,
        bytes calldata proof
    ) external view override returns (bool) {
        (bytes memory rawProof, bytes32[] memory publicInputs) = abi.decode(
            proof,
            (bytes, bytes32[])
        );

        if (publicInputs.length < MESSAGE_COEFFS_COUNT) {
            return false;
        }
        if (!_verifyPlaintextHash(publicInputs, plaintextOutputHash)) {
            return false;
        }
        return circuitVerifier.verify(rawProof, publicInputs);
    }

    function _verifyPlaintextHash(
        bytes32[] memory publicInputs,
        bytes32 plaintextOutputHash
    ) internal pure returns (bool) {
        uint256 offset = publicInputs.length - MESSAGE_COEFFS_COUNT;
        bytes memory plaintext = new bytes(MESSAGE_COEFFS_COUNT * 8);
        for (uint256 i = 0; i < MESSAGE_COEFFS_COUNT; i++) {
            uint64 coeff = uint64(uint256(publicInputs[offset + i]));
            for (uint256 j = 0; j < 8; j++) {
                plaintext[i * 8 + j] = bytes1(uint8(coeff >> (j * 8)));
            }
        }
        return keccak256(plaintext) == plaintextOutputHash;
    }
}
