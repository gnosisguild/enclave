// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {
    AdvancedChecker
} from "@excubiae/contracts/checker/AdvancedChecker.sol";

/// @title MockInputValidatorChecker.
/// @notice Enclave Input Validator
/// @dev Extends BaseChecker for input verification.
contract MockInputValidatorChecker is AdvancedChecker {
    /// @notice Validates input
    /// @param subject Address to check.
    /// @param evidence mock proof
    /// @return True if proof is valid
    function _checkMain(
        address subject,
        bytes calldata evidence
    ) internal view override returns (bool) {
        super._checkMain(subject, evidence);
        bool success;

        if (evidence.length == 3) {
            success = false;
        } else {
            success = true;
        }
        return success;
    }
}
