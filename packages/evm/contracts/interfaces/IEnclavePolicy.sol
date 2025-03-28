// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

// solhint-disable no-empty-blocks

import {
    IAdvancedPolicy
} from "@excubiae/contracts/interfaces/IAdvancedPolicy.sol";

/// @title IEnclavePolicy.
/// @notice Extends IPolicy with multi-phase validation capabilities.
interface IEnclavePolicy is IAdvancedPolicy {}
