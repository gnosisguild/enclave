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
 * @notice Decryption verifier for the fhe.rs:BFV encryption scheme. Verifies C7
 *         (decrypted_shares_aggregation) proofs on-chain by delegating to the Honk
 *         ThresholdDecryptedSharesAggregationVerifier and validating that the
 *         plaintext extracted from public inputs matches the claimed hash.
 * @dev Use this verifier when the Enclave is configured with encryptionSchemeId
 *      keccak256("fhe.rs:BFV"). Other encryption schemes will have their own verifiers.
 */
contract BfvDecryptionVerifier is IDecryptionVerifier {
    /// @dev Message is always the last 100 public inputs (100 uint64 coeffs = 800 bytes plaintext).
    ///      Layout-agnostic: works for prod and insecure circuit configs.
    uint256 constant MESSAGE_COEFFS_COUNT = 100;

    ICircuitVerifier public immutable circuitVerifier;

    constructor(address _circuitVerifier) {
        circuitVerifier = ICircuitVerifier(_circuitVerifier);
    }

    /// @inheritdoc IDecryptionVerifier
    function verify(
        bytes32 plaintextOutputHash,
        bytes memory proof
    ) external view override returns (bool success) {
        (bytes memory rawProof, bytes32[] memory publicInputs) = abi.decode(
            proof,
            (bytes, bytes32[])
        );

        if (publicInputs.length < MESSAGE_COEFFS_COUNT) {
            return false;
        }

        if (!circuitVerifier.verify(rawProof, publicInputs)) {
            return false;
        }

        if (!_verifyPlaintextHash(publicInputs, plaintextOutputHash)) {
            return false;
        }

        return true;
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
