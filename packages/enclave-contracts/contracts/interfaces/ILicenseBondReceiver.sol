// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity 0.8.28;

import {
    IERC165
} from "@openzeppelin/contracts/utils/introspection/IERC165.sol";

/**
 * @title ILicenseBondReceiver
 * @notice Optional callback surface for contracts that receive ENCL license-bond withdrawals.
 */
interface ILicenseBondReceiver is IERC165 {
    /**
     * @notice Called by BondingRegistry after a queued ENCL bond source is returned.
     * @param operator Operator whose bond source exited
     * @param amount Amount returned to the receiver
     * @param sourceId External source id supplied at bond time
     * @return selector This function's selector on success
     */
    function onLicenseBondReturned(
        address operator,
        uint256 amount,
        bytes32 sourceId
    ) external returns (bytes4 selector);

    /**
     * @notice Called by BondingRegistry after a ENCL bond source is slashed.
     * @param operator Operator whose bond source was slashed
     * @param amount Amount slashed from the source
     * @param sourceId External source id supplied at bond time
     * @return selector This function's selector on success
     */
    function onLicenseBondSlashed(
        address operator,
        uint256 amount,
        bytes32 sourceId
    ) external returns (bytes4 selector);
}
