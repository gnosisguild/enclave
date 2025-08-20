// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { Ownable } from "@oz/access/Ownable.sol";

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IBondingManager } from "../interfaces/IBondingManager.sol";

contract BondingManager is Ownable, IBondingManager {
    /// @notice Decommission delay in seconds
    uint256 public decommissionDelay;

    /// @notice Mapping of operator address to their information
    mapping(address operator => OperatorInfo info) public operators;

    /// @notice ServiceManager contract
    address public serviceManager;

    /// @notice CiphernodeRegistry contract
    ICiphernodeRegistry public ciphernodeRegistry;

    /// @notice Modifier to restrict access to ServiceManager
    modifier onlyServiceManager() {
        require(msg.sender == serviceManager, OnlyServiceManager());
        _;
    }

    constructor(
        address _serviceManager,
        address _ciphernodeRegistry,
        address _owner,
        uint256 _decommissionDelay
    ) Ownable(_owner) {
        require(_serviceManager != address(0), ZeroAddress());
        require(_ciphernodeRegistry != address(0), ZeroAddress());

        serviceManager = _serviceManager;
        ciphernodeRegistry = ICiphernodeRegistry(_ciphernodeRegistry);
        decommissionDelay = _decommissionDelay;
    }

    // ============ ServiceManager Interface ============

    function registerOperator(
        address operator,
        uint256 collateralUsd
    ) external onlyServiceManager {
        require(!operators[operator].isActive, "Already registered");

        operators[operator] = OperatorInfo({
            isActive: true,
            registeredAt: block.timestamp,
            decommissionRequestedAt: 0,
            collateralUsd: collateralUsd
        });

        emit OperatorRegistered(operator, collateralUsd);
    }

    function deregisterOperator(address operator) external onlyServiceManager {
        require(operators[operator].isActive, OperatorNotRegistered());

        operators[operator].isActive = false;
        operators[operator].decommissionRequestedAt = 0;

        emit OperatorDeregistered(operator);
    }

    function updateOperatorCollateral(
        address operator,
        uint256 newCollateralUsd
    ) external onlyServiceManager {
        require(operators[operator].isActive, OperatorNotRegistered());

        operators[operator].collateralUsd = newCollateralUsd;
    }

    // ============ Operator Interface ============

    function requestDecommission() external {
        OperatorInfo storage operatorInfo = operators[msg.sender];
        require(operatorInfo.isActive, OperatorNotRegistered());
        require(operatorInfo.decommissionRequestedAt == 0, AlreadyRequested());

        operatorInfo.decommissionRequestedAt = block.timestamp;
        emit DecommissionRequested(msg.sender, block.timestamp);
    }

    function completeDecommission(uint256[] calldata siblingNodes) external {
        OperatorInfo storage operatorInfo = operators[msg.sender];
        require(operatorInfo.isActive, OperatorNotRegistered());
        require(
            operatorInfo.decommissionRequestedAt > 0,
            DecommissionNotRequested()
        );
        require(
            block.timestamp >=
                operatorInfo.decommissionRequestedAt + decommissionDelay,
            DecommissionDelayNotPassed()
        );

        operatorInfo.isActive = false;
        operatorInfo.decommissionRequestedAt = 0;

        if (ciphernodeRegistry.isEnabled(msg.sender)) {
            ciphernodeRegistry.removeCiphernode(msg.sender, siblingNodes);
        }

        emit DecommissionCompleted(msg.sender);
    }

    // ============ View Functions ============

    function isBonded(address operator) external view returns (bool) {
        return operators[operator].isActive;
    }

    function getOperatorInfo(
        address operator
    ) external view returns (OperatorInfo memory) {
        return operators[operator];
    }

    function canCompleteDecommission(
        address operator
    ) external view returns (bool) {
        OperatorInfo memory operatorInfo = operators[operator];

        if (
            !operatorInfo.isActive || operatorInfo.decommissionRequestedAt == 0
        ) {
            return false;
        }

        return
            block.timestamp >=
            operatorInfo.decommissionRequestedAt + decommissionDelay;
    }

    function getDecommissionDelay() external view returns (uint256) {
        return decommissionDelay;
    }

    // ============ Administrative Functions ============

    function setDecommissionDelay(uint256 newDelay) external onlyOwner {
        decommissionDelay = newDelay;
        emit DecommissionDelayUpdated(newDelay);
    }

    function setServiceManager(address _serviceManager) external onlyOwner {
        require(_serviceManager != address(0), ZeroAddress());
        serviceManager = _serviceManager;
    }

    function setCiphernodeRegistry(
        address _ciphernodeRegistry
    ) external onlyOwner {
        require(_ciphernodeRegistry != address(0), ZeroAddress());
        ciphernodeRegistry = ICiphernodeRegistry(_ciphernodeRegistry);
    }
}
