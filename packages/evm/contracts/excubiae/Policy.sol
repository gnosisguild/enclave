// SPDX-License-Identifier: MIT
//  Copyright (C) 2024 Privacy & Scaling Explorations
//  Auto-generated from https://github.com/privacy-scaling-explorations/excubiae.git@96a3312455417dc1b2e0d87066661fdf8f490fac
pragma solidity ^0.8.20;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {IPolicy} from "./interfaces/IPolicy.sol";

/// @title Policy
/// @notice Implements a base policy contract that protects access to a target contract
/// @dev Inherits from OpenZeppelin's Ownable and implements IPolicy interface
///
/// This contract serves as a base for implementing specific policy checks that must be
/// satisfied before interacting with a protected target contract. It provides core
/// functionality for managing the protected target address and access control.
abstract contract Policy is IPolicy, Ownable(msg.sender) {
    /// @notice The policy-protected contract address.
    /// @dev This address can only be set once by the owner.
    /// For example, the target is a Semaphore group that requires the subject
    /// to meet certain criteria in order to join the group.
    address internal target;

    /// @notice Restricts function access to only the target contract.
    /// @dev Throws TargetOnly error if called by any other address.
    modifier onlyTarget() {
        if (msg.sender != target) revert TargetOnly();
        _;
    }

    /// @notice Sets the target contract address.
    /// @dev Can only be called once by the owner.
    /// @param _target Address of the contract to be protected by this policy.
    /// @custom:throws ZeroAddress if _target is the zero address.
    /// @custom:throws TargetAlreadySet if target has already been set.
    /// @custom:emits TargetSet when target is successfully set.
    function setTarget(address _target) external virtual onlyOwner {
        if (_target == address(0)) revert ZeroAddress();
        if (target != address(0)) revert TargetAlreadySet();

        target = _target;

        emit TargetSet(_target);
    }

    /// @notice Retrieves the current target contract address.
    /// @return address The address of the policy-protected contract.
    /// @dev Returns zero address if target hasn't been set yet.
    function getTarget() public view returns (address) {
        return target;
    }
}
