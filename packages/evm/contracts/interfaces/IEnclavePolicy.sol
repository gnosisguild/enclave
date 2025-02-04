// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {
    IAdvancedPolicy
} from "../excubiae/core/interfaces/IAdvancedPolicy.sol";
import { CheckStatus, Check } from "../excubiae/core/AdvancedChecker.sol";

/// @title IEnclavePolicy.
/// @notice Extends IPolicy with multi-phase validation capabilities.
interface IEnclavePolicy is IAdvancedPolicy {
    function enforceWithLimit(
        address subject,
        bytes[] calldata evidence,
        Check checkType
    ) external;
}
