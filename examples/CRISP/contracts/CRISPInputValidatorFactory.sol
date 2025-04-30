// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {Factory} from "@excubiae/contracts/proxy/Factory.sol";
import {CRISPInputValidator} from "./CRISPInputValidator.sol";
import {IInputValidatorFactory} from "@gnosis-guild/enclave/contracts/interfaces/IInputValidatorFactory.sol";

/// @title CRISPInputValidatorFactory
/// @notice Factory for deploying minimal proxy instances of CRISPInputValidator.
contract CRISPInputValidatorFactory is IInputValidatorFactory, Factory {
    /// @notice Initializes the factory with the CRISPInputValidator implementation.
    constructor() Factory(address(new CRISPInputValidator())) {}

    /// @notice Deploys a new CRISPInputValidator clone.
    /// @param _policyAddr Address of the associated policy contract.
    function deploy(address _policyAddr) public returns (address clone) {
        bytes memory data = abi.encode(_policyAddr);

        clone = super._deploy(data);
        CRISPInputValidator(clone).initialize();
    }
}
