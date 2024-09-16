// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IInputValidator } from "./IInputValidator.sol";
import { IE3Program } from "./IE3Program.sol";
import { IDecryptionVerifier } from "./IDecryptionVerifier.sol";

/// @title E3 struct
/// @notice This struct represents an E3 computation.
/// @param threshold M/N threshold for the committee.
/// @param startWindow Start window for the computation: index zero is minimum, index 1 is the maxium.
/// @param duration Duration of the E3.
/// @param expiration Timestamp when committee duties expire.
/// @param e3Program Address of the E3 Program contract.
/// @param computeProvider Address of the compute provider contract.
/// @param inputValidator Address of the input validator contract.
/// @param decryptionVerifier Address of the output verifier contract.
/// @param committeeId ID of the selected committee.
/// @param ciphertextOutput Encrypted output data.
/// @param plaintextOutput Decrypted output data.
struct E3 {
    uint256 seed;
    uint32[2] threshold;
    uint256[2] startWindow;
    uint256 duration;
    uint256 expiration;
    IE3Program e3Program;
    bytes e3ProgramParams;
    IInputValidator inputValidator;
    IDecryptionVerifier decryptionVerifier;
    bytes committeePublicKey;
    bytes ciphertextOutput;
    bytes plaintextOutput;
}
