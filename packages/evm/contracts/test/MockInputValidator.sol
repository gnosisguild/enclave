// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IInputValidator } from "../interfaces/IInputValidator.sol";
import { IEnclavePolicy } from "../interfaces/IEnclavePolicy.sol";
import { Clone } from "@excubiae/contracts/proxy/Clone.sol";

/// @title MockInputValidator.
/// @notice Enclave Input Validator
contract MockInputValidator is IInputValidator, Clone {
    /// @notice The policy that will be used to validate the input.
    IEnclavePolicy public policy;

    /// @notice Initializes the contract with appended bytes data for configuration.
    function _initialize() internal virtual override(Clone) {
        super._initialize();
        bytes memory data = _getAppendedBytes();
        address policyAddr = abi.decode(data, (address));

        policy = IEnclavePolicy(policyAddr);
    }

    /// @notice Validates input
    /// @param sender The account that is submitting the input.
    /// @param data The input to be verified.
    /// @return input The decoded, policy-approved application payload.
    function validate(
        address sender,
        bytes memory data
    ) external returns (bytes memory input) {
        policy.enforce(sender, data);
        input = data;
    }
}
