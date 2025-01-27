// SPDX-License-Identifier: MIT
//  Copyright (C) 2024 Privacy & Scaling Explorations
//  Auto-generated from https://github.com/privacy-scaling-explorations/excubiae.git@96a3312455417dc1b2e0d87066661fdf8f490fac
pragma solidity >=0.8.27;

import {IPolicy} from "./IPolicy.sol";
import {Check} from "./IAdvancedChecker.sol";

/// @title IAdvancedPolicy.
/// @notice Extends IPolicy with multi-phase validation capabilities.
interface IAdvancedPolicy is IPolicy {
    /// @notice Thrown when multiple main checks not allowed.
    error MainCheckAlreadyEnforced();

    /// @notice Thrown when main check attempted before pre-check.
    error PreCheckNotEnforced();

    /// @notice Thrown when post check attempted before main check.
    error MainCheckNotEnforced();

    /// @notice Thrown when pre-check validation attempted while skipped.
    error CannotPreCheckWhenSkipped();

    /// @notice Thrown when post-check validation attempted while skipped.
    error CannotPostCheckWhenSkipped();

    /// @notice Emitted when validation check succeeds.
    /// @param subject Address that passed validation.
    /// @param target Protected contract address.
    /// @param evidence Validation data.
    /// @param checkType Type of check performed.
    event Enforced(address indexed subject, address indexed target, bytes[] evidence, Check checkType);

    /// @notice Enforces validation check on subject.
    /// @dev Delegates to appropriate check method based on checkType.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    /// @param checkType Check phase to execute.
    function enforce(address subject, bytes[] calldata evidence, Check checkType) external;
}
