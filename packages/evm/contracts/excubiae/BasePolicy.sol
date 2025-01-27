// SPDX-License-Identifier: MIT
//  Copyright (C) 2024 Privacy & Scaling Explorations
//  Auto-generated from https://github.com/privacy-scaling-explorations/excubiae.git@96a3312455417dc1b2e0d87066661fdf8f490fac
pragma solidity ^0.8.20;

import {IBasePolicy} from "./interfaces/IBasePolicy.sol";
import {Policy} from "./Policy.sol";
import {BaseChecker} from "./BaseChecker.sol";

/// @title BasePolicy
/// @notice Abstract base contract for implementing specific policy checks.
/// @dev Inherits from Policy and implements IBasePolicy interface.
///
/// Provides core functionality for enforcing policy checks through a BaseChecker
/// contract. Each specific policy implementation should extend this contract
/// and implement its custom checking logic.
abstract contract BasePolicy is Policy, IBasePolicy {
    /// @notice Reference to the BaseChecker contract used for validation.
    /// @dev Immutable to ensure checker cannot be changed after deployment.
    BaseChecker public immutable BASE_CHECKER;

    /// @notice Tracks enforcement status for each subject per target.
    /// @dev Maps target => subject => enforcement status.
    mapping(address => mapping(address => bool)) public enforced;

    /// @notice Initializes the contract with a BaseChecker instance.
    /// @param _baseChecker Address of the BaseChecker contract.
    /// @dev The BaseChecker address cannot be changed after deployment.
    constructor(BaseChecker _baseChecker) {
        BASE_CHECKER = _baseChecker;
    }

    /// @notice External function to enforce policy checks.
    /// @dev Only callable by the target contract.
    /// @param subject Address to enforce the check on.
    /// @param evidence Additional data required for verification.
    /// @custom:throws AlreadyEnforced if check was previously enforced.
    /// @custom:throws UnsuccessfulCheck if the check fails.
    /// @custom:emits Enforced when check succeeds.
    function enforce(address subject, bytes[] calldata evidence) external override onlyTarget {
        _enforce(subject, evidence);
    }

    /// @notice Internal implementation of enforcement logic.
    /// @dev Performs the actual check using BASE_CHECKER.
    /// @param subject Address to enforce the check on.
    /// @param evidence Additional data required for verification.
    /// @custom:throws AlreadyEnforced if already enforced for this subject.
    /// @custom:throws UnsuccessfulCheck if BASE_CHECKER.check returns false.
    function _enforce(address subject, bytes[] memory evidence) internal {
        bool checked = BASE_CHECKER.check(subject, evidence);

        if (enforced[msg.sender][subject]) revert AlreadyEnforced();
        if (!checked) revert UnsuccessfulCheck();

        enforced[msg.sender][subject] = checked;

        emit Enforced(subject, target, evidence);
    }
}
