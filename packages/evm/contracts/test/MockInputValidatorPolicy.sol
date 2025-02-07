// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IEnclavePolicy } from "../interfaces/IEnclavePolicy.sol";
import {
    AdvancedPolicy
} from "@excubiae/contracts/src/core/policy/AdvancedPolicy.sol";
import {
    AdvancedChecker
} from "@excubiae/contracts/src/core/checker/AdvancedChecker.sol";
import {
    CheckStatus,
    Check
} from "@excubiae/contracts/src/core/interfaces/IAdvancedChecker.sol";
import "hardhat/console.sol";

/// @title BaseERC721Policy.
/// @notice Policy enforcer for Enclave Input validation.
/// @dev Extends BasePolicy with Enclave specific checks.
contract MockInputValidatorPolicy is AdvancedPolicy, IEnclavePolicy {
    error MainCalledTooManyTimes();

    uint8 public INPUT_LIMIT;

    /// @notice Initializes the contract with appended bytes data for configuration.
    /// @dev Decodes AdvancedChecker address and sets the owner.
    function _initialize() internal virtual override {
        bytes memory data = _getAppendedBytes();
        (address sender, address advCheckerAddr, uint8 inputLimit) = abi.decode(
            data,
            (address, address, uint8)
        );

        _transferOwnership(sender);

        ADVANCED_CHECKER = AdvancedChecker(advCheckerAddr);
        SKIP_PRE = true;
        SKIP_POST = true;
        ALLOW_MULTIPLE_MAIN = true;
        INPUT_LIMIT = inputLimit;
    }

    function enforceWithLimit(
        address subject,
        bytes[] calldata evidence,
        Check checkType
    ) external onlyTarget {
        CheckStatus memory status = enforced[subject];
        if (INPUT_LIMIT > 0 && status.main == INPUT_LIMIT) {
            revert MainCalledTooManyTimes();
        }

        super._enforce(subject, evidence, checkType);
    }

    /// @notice Returns policy identifier.
    /// @return Policy trait string.
    function trait() external pure returns (string memory) {
        return "MockInputValidator";
    }
}
