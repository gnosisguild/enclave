// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IInputValidator } from "./IInputValidator.sol";
import { IExecutionModule } from "./IExecutionModule.sol";
import { IComputationModule } from "./IComputationModule.sol";
import { IOutputVerifier } from "./IOutputVerifier.sol";

/// @title E3 struct
/// @notice This struct represents an E3 computation.
/// @param threshold M/N threshold for the committee.
/// @param expiration Timestamp when committee duties expire.
/// @param computationModule Address of the computation module contract.
/// @param executionModule Address of the execution module contract.
/// @param inputValidator Address of the input validator contract.
/// @param outputVerifier Address of the output verifier contract.
/// @param committeeId ID of the selected committee.
/// @param ciphertextOutput Encrypted output data.
/// @param plaintextOutput Decrypted output data.
struct E3 {
    uint32[2] threshold;
    uint256 expiration;
    IComputationModule computationModule;
    IExecutionModule executionModule;
    IInputValidator inputValidator;
    IOutputVerifier outputVerifier;
    bytes32 committeeId;
    bytes ciphertextOutput;
    bytes plaintextOutput;
}
