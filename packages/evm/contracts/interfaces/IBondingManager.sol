// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

interface IBondingManager {
    /// @notice Custom errors
    error ZeroAddress();
    error OperatorNotRegistered();
    error AlreadyRequested();
    error DecommissionNotRequested();
    error DecommissionDelayNotPassed();
    error NotAuthorizedSlasher();
    error OnlyServiceManager();

    /// @notice Operator information struct
    struct OperatorInfo {
        bool isActive;
        uint256 registeredAt;
        uint256 decommissionRequestedAt;
        uint256 collateralUsd;
    }

    /// @notice Events
    event OperatorRegistered(address indexed operator, uint256 collateralUsd);
    event OperatorDeregistered(address indexed operator);
    event DecommissionRequested(address indexed operator, uint256 requestTime);
    event DecommissionCompleted(address indexed operator);
    event DecommissionDelayUpdated(uint256 newDelay);
    event SlasherAdded(address indexed slasher);
    event SlasherRemoved(address indexed slasher);

    /**
     * @notice Register an operator after they've registered with ServiceManager
     * @param operator Address of the operator to register
     * @param collateralUsd USD value of their collateral
     */
    function registerOperator(address operator, uint256 collateralUsd) external;

    /**
     * @notice Deregister an operator
     * @param operator Address of the operator to deregister
     */
    function deregisterOperator(address operator) external;

    /**
     * @notice Request decommission from being a ciphernode
     * @dev Starts the decommission delay period
     */
    function requestDecommission() external;

    /**
     * @notice Complete decommission after delay period
     * @param siblingNodes Array of sibling node indices for registry removal
     */
    function completeDecommission(uint256[] calldata siblingNodes) external;

    /**
     * @notice Update operator's collateral value
     * @param operator Address of the operator
     * @param newCollateralUsd New collateral value in USD
     */
    function updateOperatorCollateral(
        address operator,
        uint256 newCollateralUsd
    ) external;

    /**
     * @notice Set decommission delay
     * @param newDelay New delay in seconds
     */
    function setDecommissionDelay(uint256 newDelay) external;

    /**
     * @notice Check if an operator is bonded and active
     * @param operator Address of the operator
     * @return isBonded Whether the operator is bonded
     */
    function isBonded(address operator) external view returns (bool isBonded);

    /**
     * @notice Get operator information
     * @param operator Address of the operator
     * @return info Operator information struct
     */
    function getOperatorInfo(
        address operator
    ) external view returns (OperatorInfo memory info);

    /**
     * @notice Check if an operator can complete decommission
     * @param operator Address of the operator
     * @return canDecommission Whether decommission can be completed
     */
    function canCompleteDecommission(
        address operator
    ) external view returns (bool canDecommission);

    /**
     * @notice Get decommission delay
     * @return delay Decommission delay in seconds
     */
    function getDecommissionDelay() external view returns (uint256 delay);
}
