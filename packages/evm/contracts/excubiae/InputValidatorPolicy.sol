// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IEnclavePolicy } from "../interfaces/IEnclavePolicy.sol";
import { AdvancedPolicy } from "./core/AdvancedPolicy.sol";
import { IAdvancedPolicy } from "./core/interfaces/IAdvancedPolicy.sol";
import { AdvancedChecker } from "./core/AdvancedChecker.sol";
import { CheckStatus, Check } from "./core/AdvancedChecker.sol";

/// @title BaseERC721Policy.
/// @notice Policy enforcer for Enclave Input validation.
/// @dev Extends BasePolicy with Enclave specific checks.
contract InputValidatorPolicy is AdvancedPolicy, IEnclavePolicy {
    error MainCalledTooManyTimes();

    /// @notice Checker contract reference.
    AdvancedChecker public immutable CHECKER;
    uint8 public immutable INPUT_LIMIT;

    /// @notice Initializes with checker contract.
    /// @param _checker Checker contract address.
    /// @param _inputLimit The maximum amount of times the input can be enforced
    constructor(
        AdvancedChecker _checker,
        uint8 _inputLimit
    ) AdvancedPolicy(_checker, true, true, true) {
        CHECKER = AdvancedChecker(_checker);
        INPUT_LIMIT = _inputLimit;
    }

    function enforceWithLimit(
        address subject,
        bytes[] calldata evidence,
        Check checkType
    ) external onlyTarget {
        CheckStatus memory status = enforced[msg.sender][subject];
        if (INPUT_LIMIT > 0 && status.main == INPUT_LIMIT) {
            revert MainCalledTooManyTimes();
        }

        super._enforce(subject, evidence, checkType);
    }

    /// @notice Returns policy identifier.
    /// @return Policy trait string.
    function trait() external pure returns (string memory) {
        return "InputValidator";
    }
}
