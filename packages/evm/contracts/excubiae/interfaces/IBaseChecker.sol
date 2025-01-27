// SPDX-License-Identifier: MIT
//  Copyright (C) 2024 Privacy & Scaling Explorations
//  Auto-generated from https://github.com/privacy-scaling-explorations/excubiae.git@96a3312455417dc1b2e0d87066661fdf8f490fac
pragma solidity ^0.8.20;

import {IChecker} from "./IChecker.sol";

/// @title IBaseChecker.
/// @notice Defines base validation functionality.
interface IBaseChecker is IChecker {
    /// @notice Validates subject against evidence.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    /// @return checked True if validation passes.
    function check(address subject, bytes[] calldata evidence) external view returns (bool checked);
}
