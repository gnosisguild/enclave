// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

// solhint-disable no-empty-blocks

import { IBasePolicy } from "@excubiae/contracts/interfaces/IBasePolicy.sol";

/// @title IEnclavePolicy.
/// @notice Extends IPolicy with basic validation and enforcement capabilities.
interface IEnclavePolicy is IBasePolicy {
    /**
     * @dev MUST revert under the same conditions as `enforce`.
     * @param subject  the account that is submitting the input
     * @param evidence the same blob that `enforce` expects
     * @return vote    the decoded, policy-approved application payload
     */
    function validate(
        address subject,
        bytes calldata evidence
    ) external returns (bytes memory vote);
}
