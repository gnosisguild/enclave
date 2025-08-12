// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";

contract BondingManager is Ownable {
    /// @notice Decommission delay in seconds
    uint256 public decommissionDelay;

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

    // ============ View Functions ============

    function isBonded(address operator) external view returns (bool) {
        return ciphernodeRegistry.isEnabled(operator);
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
