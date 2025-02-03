// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { BaseChecker } from "../excubiae/core/BaseChecker.sol";

/// @title MockInputValidatorChecker.
/// @notice Enclave Input Validator
/// @dev Extends BaseChecker for input verification.
contract MockInputValidatorChecker is BaseChecker {
    /// @param _verifiers Array of addresses for existing verification contracts.
    constructor(address[] memory _verifiers) BaseChecker(_verifiers) {}

    /// @notice Validates input
    /// @param subject Address to check.
    /// @param evidence mock proof
    /// @return True if proof is valid
    function _check(
        address subject,
        bytes[] calldata evidence
    ) internal view override returns (bool) {
        super._check(subject, evidence);
        bool success;

        if (evidence[0].length == 3) {
            success = false;
        } else {
            success = true;
        }

        return success;
    }
}
