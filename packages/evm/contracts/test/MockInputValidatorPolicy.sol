// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IEnclavePolicy } from "../interfaces/IEnclavePolicy.sol";
import { AdvancedPolicy } from "@excubiae/contracts/policy/AdvancedPolicy.sol";
import {
    AdvancedChecker
} from "@excubiae/contracts/checker/AdvancedChecker.sol";
import { Check } from "@excubiae/contracts/interfaces/IAdvancedChecker.sol";

/// @title BaseERC721Policy.
/// @notice Policy enforcer for Enclave Input validation.
/// @dev Extends BasePolicy with Enclave specific checks.
contract MockInputValidatorPolicy is AdvancedPolicy, IEnclavePolicy {
    error MainCalledTooManyTimes();

    uint8 public inputLimit;
    mapping(address subject => uint8 count) public enforced;

    /// @notice Initializes the contract with appended bytes data for configuration.
    /// @dev Decodes AdvancedChecker address and sets the owner.
    function _initialize() internal virtual override {
        bytes memory data = _getAppendedBytes();
        (address sender, address advCheckerAddr, uint8 _inputLimit) = abi
            .decode(data, (address, address, uint8));
        _transferOwnership(sender);

        ADVANCED_CHECKER = AdvancedChecker(advCheckerAddr);
        SKIP_PRE = true;
        SKIP_POST = true;
        inputLimit = _inputLimit;
    }

    function _enforce(
        address subject,
        bytes calldata evidence,
        Check checkType
    ) internal override(AdvancedPolicy) onlyTarget {
        uint256 status = enforced[subject];
        if (inputLimit > 0 && status == inputLimit) {
            revert MainCalledTooManyTimes();
        }

        super._enforce(subject, evidence, checkType);
        enforced[subject]++;
    }

    /// @notice Returns policy identifier.
    /// @return Policy trait string.
    function trait() external pure returns (string memory) {
        return "MockInputValidator";
    }
}
