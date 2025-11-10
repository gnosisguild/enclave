// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IE3Program } from "./IE3Program.sol";
import { IDecryptionVerifier } from "./IDecryptionVerifier.sol";

/**
 * @title E3
 * @notice Represents a complete E3 (Encrypted Execution Environment) computation request and its lifecycle
 * @dev This struct tracks all parameters, state, and results of an encrypted computation
 *      from request through completion
 * @param seed Random seed for committee selection and computation initialization
 * @param threshold M/N threshold for the committee (M required out of N total members)
 * @param requestBlock Block number when the E3 computation was requested
 * @param startWindow Start window for the computation: index 0 is minimum block, index 1 is the maximum block
 * @param duration Duration of the E3 computation in blocks or time units
 * @param expiration Timestamp when committee duties expire and computation is considered failed
 * @param encryptionSchemeId Identifier for the encryption scheme used in this computation
 * @param e3Program Address of the E3 Program contract that validates and verifies the computation
 * @param e3ProgramParams ABI encoded computation parameters specific to the E3 program
 * @param customParams Arbitrary ABI-encoded application-defined parameters.
 * @param decryptionVerifier Address of the output verifier contract for decryption verification
 * @param committeePublicKey The public key of the selected committee for this computation
 * @param ciphertextOutput Hash of the encrypted output data produced by the computation
 * @param plaintextOutput Decrypted output data after committee decryption
 */
struct E3 {
    uint256 seed;
    uint32[2] threshold;
    uint256 requestBlock;
    uint256[2] startWindow;
    uint256 duration;
    uint256 expiration;
    bytes32 encryptionSchemeId;
    IE3Program e3Program;
    bytes e3ProgramParams;
    bytes customParams;
    IDecryptionVerifier decryptionVerifier;
    bytes32 committeePublicKey;
    bytes32 ciphertextOutput;
    bytes plaintextOutput;
}
