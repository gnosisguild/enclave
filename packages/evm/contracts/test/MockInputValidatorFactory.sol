// SPDX-License-Identifier: MIT
pragma solidity >=0.8.27;

import { MockInputValidator } from "./MockInputValidator.sol";
import {
    IInputValidatorFactory
} from "../interfaces/IInputValidatorFactory.sol";
import { Factory } from "@excubiae/contracts/proxy/Factory.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";

/// @title MockInputValidatorFactory
/// @notice Factory for deploying minimal proxy instances of MockInputValidator.
/// @dev Encodes configuration data for multi-phase policy validation.
contract MockInputValidatorFactory is
    IInputValidatorFactory,
    Factory,
    Ownable(msg.sender)
{
    /// @notice Initializes the factory with the MockInputValidator implementation.
    constructor() Factory(address(new MockInputValidator())) {}

    /// @notice Deploys a new MockInputValidator clone.
    /// @param _policyAddr Address of the associated policy contract.
    function deploy(
        address _policyAddr
    ) public onlyOwner returns (address clone) {
        bytes memory data = abi.encode(_policyAddr);

        clone = super._deploy(data);
        MockInputValidator(clone).initialize();
    }
}
