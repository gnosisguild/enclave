// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {Factory} from "@excubiae/contracts/proxy/Factory.sol";
import {CRISPPolicy} from "./CRISPPolicy.sol";

/// @title CRISPPolicyFactory
/// @notice Factory for deploying minimal proxy instances of CRISPPolicy.
/// @dev Encodes configuration data for multi-phase policy validation.
contract CRISPPolicyFactory is Factory {
    /// @notice Initializes the factory with the CRISPPolicy implementation.
    constructor() Factory(address(new CRISPPolicy())) {}

    /// @notice Deploys a new CRISPPolicy clone.
    /// @param _checkerAddr Address of the associated checker contract.
    /// @param _inputLimit Maximum number of times that input may be submitted.
    function deploy(
        address _checkerAddr,
        uint8 _inputLimit
    ) public returns (address clone) {
        bytes memory data = abi.encode(msg.sender, _checkerAddr, _inputLimit);

        clone = super._deploy(data);
        CRISPPolicy(clone).initialize();
    }
}
