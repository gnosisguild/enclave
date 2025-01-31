// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { BasePolicy } from "./core/BasePolicy.sol";
import { BaseChecker } from "./core/BaseChecker.sol";

/// @title BaseERC721Policy.
/// @notice Policy enforcer for Enclave Input validation.
/// @dev Extends BasePolicy with Enclave specific checks.
contract InputValidatorPolicy is BasePolicy {
    /// @notice Checker contract reference.
    BaseChecker public immutable CHECKER;

    /// @notice Initializes with checker contract.
    /// @param _checker Checker contract address.
    constructor(BaseChecker _checker) BasePolicy(_checker) {
        CHECKER = BaseChecker(_checker);
    }

    /// @notice Returns policy identifier.
    /// @return Policy trait string.
    function trait() external pure returns (string memory) {
        return "InputValidator";
    }
}
