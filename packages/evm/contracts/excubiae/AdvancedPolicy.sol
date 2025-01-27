// SPDX-License-Identifier: MIT
// Auto-generated from https://github.com/privacy-scaling-explorations/excubiae.git@96a3312455417dc1b2e0d87066661fdf8f490fac
pragma solidity ^0.8.20;

import {Policy} from "./Policy.sol";
import {IAdvancedPolicy, Check} from "./interfaces/IAdvancedPolicy.sol";
import {AdvancedChecker, CheckStatus} from "./AdvancedChecker.sol";

/// @title AdvancedPolicy.
/// @notice Implements advanced policy checks with pre, main, and post validation stages.
/// @dev Extends Policy contract with multi-stage validation capabilities.
abstract contract AdvancedPolicy is IAdvancedPolicy, Policy {
    /// @notice Reference to the validation checker contract.
    /// @dev Immutable to ensure checker cannot be changed after deployment.
    AdvancedChecker public immutable ADVANCED_CHECKER;

    /// @notice Controls whether pre-condition checks are required.
    bool public immutable SKIP_PRE;

    /// @notice Controls whether post-condition checks are required.
    bool public immutable SKIP_POST;

    /// @notice Controls whether main check can be executed multiple times.
    bool public immutable ALLOW_MULTIPLE_MAIN;

    /// @notice Tracks validation status for each subject per target.
    /// @dev Maps target => subject => CheckStatus.
    mapping(address => mapping(address => CheckStatus)) public enforced;

    /// @notice Initializes contract with an AdvancedChecker instance and checks configs.
    /// @param _advancedChecker Address of the AdvancedChecker contract.
    /// @param _skipPre Skip pre-condition validation.
    /// @param _skipPost Skip post-condition validation.
    /// @param _allowMultipleMain Allow multiple main validations.
    constructor(AdvancedChecker _advancedChecker, bool _skipPre, bool _skipPost, bool _allowMultipleMain) {
        ADVANCED_CHECKER = _advancedChecker;
        SKIP_PRE = _skipPre;
        SKIP_POST = _skipPost;
        ALLOW_MULTIPLE_MAIN = _allowMultipleMain;
    }

    /// @notice Enforces policy check for a subject.
    /// @dev Only callable by target contract.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    /// @param checkType Type of check (PRE, MAIN, POST).
    function enforce(address subject, bytes[] calldata evidence, Check checkType) external override onlyTarget {
        _enforce(subject, evidence, checkType);
    }

    /// @notice Internal check enforcement logic.
    /// @dev Handles different check types and their dependencies.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    /// @param checkType Type of check to perform.
    /// @custom:throws CannotPreCheckWhenSkipped If PRE check attempted when skipped.
    /// @custom:throws CannotPostCheckWhenSkipped If POST check attempted when skipped.
    /// @custom:throws UnsuccessfulCheck If validation fails.
    /// @custom:throws AlreadyEnforced If check was already completed.
    /// @custom:throws PreCheckNotEnforced If PRE check is required but not done.
    /// @custom:throws MainCheckNotEnforced If MAIN check is required but not done.
    /// @custom:throws MainCheckAlreadyEnforced If multiple MAIN checks not allowed.
    function _enforce(address subject, bytes[] calldata evidence, Check checkType) internal {
        if (!ADVANCED_CHECKER.check(subject, evidence, checkType)) {
            revert UnsuccessfulCheck();
        }

        CheckStatus storage status = enforced[msg.sender][subject];

        // Handle PRE check.
        if (checkType == Check.PRE) {
            if (SKIP_PRE) revert CannotPreCheckWhenSkipped();
            if (status.pre) {
                revert AlreadyEnforced();
            }

            status.pre = true;
        } else if (checkType == Check.POST) {
            // Handle POST check.
            if (SKIP_POST) revert CannotPostCheckWhenSkipped();
            if (status.main == 0) {
                revert MainCheckNotEnforced();
            }
            if (status.post) {
                revert AlreadyEnforced();
            }
            status.post = true;
        } else {
            // Handle MAIN check.
            if (!SKIP_PRE && !status.pre) {
                revert PreCheckNotEnforced();
            }
            if (!ALLOW_MULTIPLE_MAIN && status.main > 0) {
                revert MainCheckAlreadyEnforced();
            }
            status.main += 1;
        }

        emit Enforced(subject, target, evidence, checkType);
    }
}
