// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {IInputValidator} from "@gnosis-guild/enclave/contracts/interfaces/IInputValidator.sol";
import {IBasePolicy} from "@excubiae/contracts/interfaces/IBasePolicy.sol";
import {Clone} from "@excubiae/contracts/proxy/Clone.sol";
import {IVerifier} from "./CRISPVerifier.sol";

/// @title CRISPInputValidator.
/// @notice Enclave Input Validator
contract CRISPInputValidator is IInputValidator, Clone {
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

        (address policyAddr, address verifierAddr) = abi.decode(
            _getAppendedBytes(),
            (address, address)
        );
        policy = IBasePolicy(policyAddr);
        noirVerifier = IVerifier(verifierAddr);
    }

    /// @notice Validates input
    /// @param sender The account that is submitting the input.
    /// @param data The input to be verified.
    /// @return input The decoded, policy-approved application payload.
    function validate(
        address sender,
        bytes memory data
    ) external returns (bytes memory input) {
        if (data.length == 0) revert EmptyInputData();

        (
            bytes memory semaphoreProof,
            bytes memory noirProof,
            bytes32[] memory noirPublicInputs,
            bytes memory vote
        ) = abi.decode(data, (bytes, bytes, bytes32[], bytes));

        // Reverts if the semaphore proof is invalid
        policy.enforce(sender, semaphoreProof);

        // Reverts if noir proof is invalid
        if (!noirVerifier.verify(noirProof, noirPublicInputs))
            revert InvalidNoirProof();

        input = vote;
    }
}
