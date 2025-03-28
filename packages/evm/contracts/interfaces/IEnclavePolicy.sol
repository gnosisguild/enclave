// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {
    IAdvancedPolicy
} from "@excubiae/contracts/interfaces/IAdvancedPolicy.sol";
import { Check } from "@excubiae/contracts/checker/AdvancedChecker.sol";

/// @title IEnclavePolicy.
/// @notice Extends IPolicy with multi-phase validation capabilities.
interface IEnclavePolicy is IAdvancedPolicy {

}
