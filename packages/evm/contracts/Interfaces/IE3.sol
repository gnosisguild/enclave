// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IInputValidator } from "./IInputValidator.sol";
import { IExecutionModule } from "./IExecutionModule.sol";
import { IComputationModule } from "./IComputationModule.sol";
import { IOutputVerifier } from "./IOutputVerifier.sol";

struct E3 {
    uint32[2] threshold; // M/N threshold for the committee.
    uint256 expiration; // timestamp when committee duties expire.
    IComputationModule computationModule; // address of the computation module contract.
    IExecutionModule executionModule; // address of the execution module contract.
    IInputValidator inputValidator; // address of the input validator contract.
    IOutputVerifier outputVerifier; // address of the output verifier contract.
    bytes32 committeeId; // ID of the selected committee.
    bytes ciphertextOutput; // encrypted output data.
    bytes plaintextOutput; // decrypted output data.
}
