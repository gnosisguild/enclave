// SPDX-License-Identifier: MIT
//  Copyright (C) 2024 Privacy & Scaling Explorations
//  Auto-generated from https://github.com/privacy-scaling-explorations/excubiae.git@96a3312455417dc1b2e0d87066661fdf8f490fac
pragma solidity ^0.8.20;

import {IChecker} from "./IChecker.sol";

/// @title Check.
/// @notice Defines validation phases in the AdvancedChecker system.
/// @custom:values PRE - Pre-condition validation.
///                MAIN - Primary validation.
///                POST - Post-condition validation.
enum Check {
    PRE,
    MAIN,
    POST
}

/// @notice Tracks validation status for pre, main, and post checks.
/// @dev Used to maintain check state in AdvancedPolicy.
struct CheckStatus {
    /// @dev Pre-check completion status.
    bool pre;
    /// @dev Number of completed main checks.
    uint8 main;
    /// @dev Post-check completion status.
    bool post;
}

/// @title IAdvancedChecker.
/// @notice Defines multi-phase validation system interface.
/// @dev Implement this for custom validation logic with pre/main/post checks.
interface IAdvancedChecker is IChecker {
    /// @notice Validates subject against specified check type.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    /// @param checkType Check phase to execute.
    /// @return checked True if validation passes.
    function check(address subject, bytes[] calldata evidence, Check checkType) external view returns (bool checked);
}
