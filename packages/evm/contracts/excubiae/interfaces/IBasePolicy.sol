// SPDX-License-Identifier: MIT
// Auto-generated from https://github.com/privacy-scaling-explorations/excubiae.git@96a3312455417dc1b2e0d87066661fdf8f490fac
pragma solidity ^0.8.20;

import {IPolicy} from "./IPolicy.sol";

/// @title IBasePolicy.
/// @notice Extends IPolicy with basic validation capabilities.
interface IBasePolicy is IPolicy {
    /// @notice Emitted when validation succeeds.
    /// @param subject Address that passed validation.
    /// @param target Protected contract address.
    /// @param evidence Validation data.
    event Enforced(address indexed subject, address indexed target, bytes[] evidence);

    /// @notice Enforces validation check on subject.
    /// @param subject Address to validate.
    /// @param evidence Validation data.
    function enforce(address subject, bytes[] calldata evidence) external;
}
