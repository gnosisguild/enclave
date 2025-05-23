// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {IInputValidator} from "@gnosis-guild/enclave/contracts/interfaces/IInputValidator.sol";
import {IBasePolicy} from "@excubiae/contracts/interfaces/IBasePolicy.sol";
import {Clone} from "@excubiae/contracts/proxy/Clone.sol";

/// @title CRISPInputValidator.
/// @notice Enclave Input Validator
contract CRISPInputValidator is IInputValidator, Clone {
    /// @notice The policy that will be used to validate the input.
    IBasePolicy internal policy;

    /// @notice The error emitted when the input data is empty.
    error EmptyInputData();
    /// @notice The error emitted when the input data is invalid.
    error InvalidInputData(bytes reason);

    /// @notice Initializes the contract with appended bytes data for configuration.
    function _initialize() internal virtual override(Clone) {
        super._initialize();
        bytes memory data = _getAppendedBytes();
        address policyAddr = abi.decode(data, (address));

        policy = IBasePolicy(policyAddr);
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

        (bytes memory proofBytes, bytes memory vote) = abi.decode(
            data,
            (bytes, bytes)
        );

        // Reverts if the proof is invalid
        policy.enforce(sender, proofBytes);

        input = vote;
    }
}
