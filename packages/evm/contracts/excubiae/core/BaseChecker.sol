// SPDX-License-Identifier: MIT
//  Copyright (C) 2024 Privacy & Scaling Explorations
//  Auto-generated from https://github.com/privacy-scaling-explorations/excubiae.git@96a3312455417dc1b2e0d87066661fdf8f490fac
pragma solidity >=0.8.27;

import {IBaseChecker} from "./interfaces/IBaseChecker.sol";
import {Checker} from "./Checker.sol";

/// @title BaseChecker
/// @notice Abstract base contract for implementing validation checks.
/// @dev Provides a standardized interface for implementing custom validation logic
/// through the internal _check method.
abstract contract BaseChecker is Checker, IBaseChecker {
    constructor(address[] memory _verifiers) Checker(_verifiers) {}

    /// @notice Validates evidence for a given subject address.
    /// @dev External view function that delegates to internal _check implementation.
    /// @param subject Address to validate.
    /// @param evidence Custom validation data.
    /// @return checked Boolean indicating if the check passed.
    function check(address subject, bytes[] calldata evidence) external view override returns (bool checked) {
        return _check(subject, evidence);
    }

    /// @notice Internal validation logic implementation.
    /// @dev Must be implemented by derived contracts.
    /// @param subject Address to validate.
    /// @param evidence Custom validation data.
    /// @return checked Boolean indicating if the check passed.
    function _check(address subject, bytes[] calldata evidence) internal view virtual returns (bool checked) {}
}
