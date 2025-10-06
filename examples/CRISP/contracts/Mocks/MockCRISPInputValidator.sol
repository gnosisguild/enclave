// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {IInputValidator} from "@enclave-e3/contracts/contracts/interfaces/IInputValidator.sol";
import {IBasePolicy} from "@excubiae/contracts/interfaces/IBasePolicy.sol";
import {Clone} from "@excubiae/contracts/proxy/Clone.sol";
import {IVerifier} from "../CRISPVerifier.sol";

/// @title MockCRISPInputValidator.
/// @notice Mock Enclave Input Validator
contract MockCRISPInputValidator is IInputValidator, Clone {
    /// @notice The policy that will be used to validate the input.
    IBasePolicy internal policy;

    /// @notice The verifier that will be used to validate the input.
    IVerifier internal noirVerifier;

    /// @notice The error emitted when the input data is empty.
    error EmptyInputData();
    /// @notice The error emitted when the input data is invalid.
    error InvalidInputData(bytes reason);
    /// @notice The error emitted when the Noir proof is invalid.
    error InvalidNoirProof();

    /// @notice Initializes the contract with appended bytes data for configuration.
    function _initialize() internal virtual override(Clone) {
        super._initialize();

        (address policyAddr, address verifierAddr) = abi.decode(_getAppendedBytes(), (address, address));
        policy = IBasePolicy(policyAddr);
        noirVerifier = IVerifier(verifierAddr);
    }

    /// @notice Validates input
    /// @param sender The account that is submitting the input.
    /// @param data The input to be verified.
    /// @return input The decoded, policy-approved application payload.
    function validate(address sender, bytes memory data) external returns (bytes memory input) {
        if (data.length == 0) revert EmptyInputData();

        (,,, bytes memory vote) = abi.decode(data, (bytes, bytes, bytes32[], bytes));

        input = vote;
    }
}
