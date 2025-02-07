// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import { MockInputValidatorPolicy } from "./MockInputValidatorPolicy.sol";
import { IEnclavePolicyFactory } from "../interfaces/IEnclavePolicyFactory.sol";
import { Factory } from "@excubiae/contracts/src/core/proxy/Factory.sol";
import "hardhat/console.sol";

/// @title AdvancedERC721PolicyFactory
/// @notice Factory for deploying minimal proxy instances of AdvancedERC721Policy.
/// @dev Encodes configuration data for multi-phase policy validation.
contract MockInputValidatorPolicyFactory is IEnclavePolicyFactory, Factory {
    /// @notice Initializes the factory with the AdvancedERC721Policy implementation.
    constructor() Factory(address(new MockInputValidatorPolicy())) {}

    /// @notice Deploys a new AdvancedERC721Policy clone.
    /// @param _checkerAddr Address of the associated checker contract.
    /// @param _inputLimit Maximum number of times that input may be submitted.
    function deploy(
        address _checkerAddr,
        uint8 _inputLimit
    ) public returns (address clone) {
        bytes memory data = abi.encode(msg.sender, _checkerAddr, _inputLimit);

        clone = super._deploy(data);
        MockInputValidatorPolicy(clone).initialize();
    }
}
