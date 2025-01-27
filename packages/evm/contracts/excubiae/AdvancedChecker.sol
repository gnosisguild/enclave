// SPDX-License-Identifier: MIT
//  Copyright (C) 2024 Privacy & Scaling Explorations
//  Auto-generated from https://github.com/privacy-scaling-explorations/excubiae.git@96a3312455417dc1b2e0d87066661fdf8f490fac
pragma solidity >=0.8.27;

import {IAdvancedChecker, Check, CheckStatus} from "./interfaces/IAdvancedChecker.sol";
import {Checker} from "./Checker.sol";

/// @title AdvancedChecker.
/// @notice Multi-phase validation checker with pre, main, and post checks.
/// @dev Base contract for implementing complex validation logic with configurable check phases.
abstract contract AdvancedChecker is IAdvancedChecker, Checker {
    constructor(address[] memory _verifiers) Checker(_verifiers) {}

    /// @notice Entry point for validation checks.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    /// @param checkType Type of check (PRE, MAIN, POST).
    /// @return checked Validation result.
    function check(
        address subject,
        bytes[] calldata evidence,
        Check checkType
    ) external view override returns (bool checked) {
        return _check(subject, evidence, checkType);
    }

    /// @notice Core validation logic router.
    /// @dev Directs to appropriate check based on type and configuration.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    /// @param checkType Check type to perform.
    /// @return checked Validation result.
    function _check(address subject, bytes[] calldata evidence, Check checkType) internal view returns (bool checked) {
        if (checkType == Check.PRE) {
            return _checkPre(subject, evidence);
        }

        if (checkType == Check.POST) {
            return _checkPost(subject, evidence);
        }

        return _checkMain(subject, evidence);
    }

    /// @notice Pre-condition validation implementation.
    /// @dev Override to implement pre-check logic.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    /// @return checked Validation result.
    function _checkPre(address subject, bytes[] calldata evidence) internal view virtual returns (bool checked) {}

    /// @notice Main validation implementation.
    /// @dev Override to implement main check logic.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    /// @return checked Validation result.
    function _checkMain(address subject, bytes[] calldata evidence) internal view virtual returns (bool checked) {}

    /// @notice Post-condition validation implementation.
    /// @dev Override to implement post-check logic.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    /// @return checked Validation result.
    function _checkPost(address subject, bytes[] calldata evidence) internal view virtual returns (bool checked) {}
}
