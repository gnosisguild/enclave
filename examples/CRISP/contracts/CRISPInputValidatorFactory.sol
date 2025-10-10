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
    constructor(address inputValidator) Factory(inputValidator) {}

    /// @notice Deploys a new CRISPInputValidator clone.
    /// @param _verifierAddr Address of the associated verifier contract.
    function deploy(
        address _verifierAddr,
        address _owner
    ) public returns (address clone) {
        bytes memory data = abi.encode(_verifierAddr, _owner);

        clone = super._deploy(data);
        CRISPInputValidator(clone).initialize();
    }
}
