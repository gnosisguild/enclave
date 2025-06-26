// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {Factory} from "@excubiae/contracts/proxy/Factory.sol";
import {CRISPPolicyNoir} from "./CRISPPolicyNoir.sol";

/// @title CRISPPolicyNoirFactory
/// @notice Factory contract for deploying minimal proxy instances of CRISPPolicyNoir.
/// @dev Utilizes the Factory pattern to streamline deployment of CRISPPolicyNoir clones with configuration data.
contract CRISPPolicyNoirFactory is Factory {
    /// @notice Initializes the factory with the CRISPPolicyNoir implementation.
    /// @dev The constructor sets the CRISPPolicyNoir contract as the implementation for cloning.
    constructor() Factory(address(new CRISPPolicyNoir())) {}

    /// @notice Deploys a new CRISPPolicyNoir clone with the specified parameters.
    /// @dev Encodes the owner, checker address, and input limit as initialization data for the clone.
    /// @param owner Address to be set as the owner of the policy.
    /// @param checker Address of the BaseChecker contract.
    /// @param inputLimit Maximum number of inputs allowed per subject.
    /// @return clone The address of the newly deployed CRISPPolicyNoir clone.
    function deploy(
        address owner,
        address checker,
        uint8 inputLimit
    ) public returns (address clone) {
        bytes memory data = abi.encode(owner, checker, inputLimit);

        clone = super._deploy(data);

        CRISPPolicyNoir(clone).initialize();
    }
}