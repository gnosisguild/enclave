// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {Factory} from "@excubiae/contracts/proxy/Factory.sol";
import {CRISPInputValidator} from "./CRISPInputValidator.sol";

/// @title CRISPInputValidatorFactory
/// @notice Factory for deploying minimal proxy instances of CRISPInputValidator.
contract CRISPInputValidatorFactory is Factory {
    /// @notice Initializes the factory with the CRISPInputValidator implementation.
    constructor() Factory(address(new CRISPInputValidator())) {}

    /// @notice Deploys a new CRISPInputValidator clone.
    /// @param _policyAddr Address of the associated policy contract.
    /// @param _verifierAddr Address of the associated verifier contract.
    function deploy(
        address _policyAddr,
        address _verifierAddr
    ) public returns (address clone) {
        bytes memory data = abi.encode(_policyAddr, _verifierAddr);

        clone = super._deploy(data);
        CRISPInputValidator(clone).initialize();
    }
}
