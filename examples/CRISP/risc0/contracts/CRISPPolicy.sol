// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {IEnclavePolicy} from "@gnosis-guild/enclave/contracts/interfaces/IEnclavePolicy.sol";
import {AdvancedPolicy} from "@excubiae/contracts/policy/AdvancedPolicy.sol";
import {AdvancedChecker} from "@excubiae/contracts/checker/AdvancedChecker.sol";
import {Check} from "@excubiae/contracts/interfaces/IAdvancedChecker.sol";

/// @title BaseERC721Policy.
/// @notice Policy enforcer for Enclave Input validation.
/// @dev Extends BasePolicy with Enclave specific checks.
contract CRISPPolicy is AdvancedPolicy, IEnclavePolicy {
    error MainCalledTooManyTimes();
    error InvalidInitializationAddress();

    uint8 public inputLimit;
    mapping(address subject => uint8 count) public enforced;

    /// @notice Constructor to initialize the policy directly upon deployment.
    /// @param _enclave The address of the Enclave contract that will call enforce (becomes the 'guarded' target).
    /// @param _checker The address of the AdvancedChecker contract to use.
    /// @param _inputLimit The maximum number of times enforce can be called per subject.
    constructor(address _enclave, address _checker, uint8 _inputLimit) {
        if (_checker == address(0) || _enclave == address(0))
            revert InvalidInitializationAddress();
        guarded = _enclave;
        _transferOwnership(_enclave);
        ADVANCED_CHECKER = AdvancedChecker(_checker);
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
        return "CRISPPolicy";
    }
}
