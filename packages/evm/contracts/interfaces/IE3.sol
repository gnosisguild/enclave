// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IInputValidator } from "./IInputValidator.sol";
import { IComputeProvider } from "./IComputeProvider.sol";
import { IE3Program } from "./IE3Program.sol";
import { IOutputVerifier } from "./IOutputVerifier.sol";

/// @title E3 struct
/// @notice This struct represents an E3 computation.
/// @param threshold M/N threshold for the committee.
/// @param startWindow Start window for the computation: index zero is minimum, index 1 is the maxium.
/// @param duration Duration of the E3.
/// @param expiration Timestamp when committee duties expire.
/// @param computationModule Address of the E3 Program contract.
/// @param computeProvider Address of the compute provider contract.
/// @param inputValidator Address of the input validator contract.
/// @param outputVerifier Address of the output verifier contract.
/// @param committeeId ID of the selected committee.
/// @param ciphertextOutput Encrypted output data.
/// @param plaintextOutput Decrypted output data.
struct E3 {
    uint32[2] threshold;
    uint256[2] startWindow;
    uint256 duration;
    uint256 expiration;
    IE3Program computationModule;
    IComputeProvider computeProvider;
    IInputValidator inputValidator;
    IOutputVerifier outputVerifier;
    bytes committeePublicKey;
    bytes ciphertextOutput;
    bytes plaintextOutput;
}
