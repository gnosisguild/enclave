// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { AdvancedPolicy } from "./core/AdvancedPolicy.sol";
import { AdvancedChecker } from "./core/AdvancedChecker.sol";

/// @title BaseERC721Policy.
/// @notice Policy enforcer for Enclave Input validation.
/// @dev Extends BasePolicy with Enclave specific checks.
contract InputValidatorPolicy is AdvancedPolicy {
    /// @notice Checker contract reference.
    AdvancedChecker public immutable CHECKER;

    /// @notice Initializes with checker contract.
    /// @param _checker Checker contract address.
    constructor(
        AdvancedChecker _checker
    ) AdvancedPolicy(_checker, true, true, true) {
        CHECKER = AdvancedChecker(_checker);
    }

    /// @notice Returns policy identifier.
    /// @return Policy trait string.
    function trait() external pure returns (string memory) {
        return "InputValidator";
    }
}
