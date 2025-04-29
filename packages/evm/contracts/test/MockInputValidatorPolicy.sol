// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IEnclavePolicy } from "../interfaces/IEnclavePolicy.sol";
import { BasePolicy } from "@excubiae/contracts/policy/BasePolicy.sol";
import { BaseChecker } from "@excubiae/contracts/checker/BaseChecker.sol";

/// @title BaseERC721Policy.
/// @notice Policy enforcer for Enclave Input validation.
/// @dev Extends BasePolicy with Enclave specific checks.
contract MockInputValidatorPolicy is BasePolicy, IEnclavePolicy {
    error MainCalledTooManyTimes();

    uint8 public inputLimit;
    mapping(address subject => uint8 count) public enforced;

    /// @notice Initializes the contract with appended bytes data for configuration.
    /// @dev Decodes AdvancedChecker address and sets the owner.
    function _initialize() internal virtual override {
        bytes memory data = _getAppendedBytes();
        (address sender, address baseCheckerAddr, uint8 _inputLimit) = abi
            .decode(data, (address, address, uint8));
        _transferOwnership(sender);

        BASE_CHECKER = BaseChecker(baseCheckerAddr);
        inputLimit = _inputLimit;
    }

    function validate(
        address subject,
        bytes calldata evidence
    ) external override onlyTarget returns (bytes memory vote) {
        _enforce(subject, evidence);
        return vote;
    }

    function _enforce(
        address subject,
        bytes calldata evidence
    ) internal override(BasePolicy) onlyTarget {
        uint256 status = enforced[subject];
        if (inputLimit > 0 && status == inputLimit) {
            revert MainCalledTooManyTimes();
        }

        super._enforce(subject, evidence);
        enforced[subject]++;
    }

    /// @notice Returns policy identifier.
    /// @return Policy trait string.
    function trait() external pure returns (string memory) {
        return "MockInputValidator";
    }
}
