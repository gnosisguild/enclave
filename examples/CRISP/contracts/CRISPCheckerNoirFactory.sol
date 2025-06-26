// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {Factory} from "@excubiae/contracts/proxy/Factory.sol";
import {CRISPCheckerNoir} from "./CRISPCheckerNoir.sol";

/// @title CRISPCheckerNoirFactory
/// @notice Factory contract for deploying minimal proxy instances of CRISPCheckerNoir.
/// @dev Utilizes the Factory pattern to streamline deployment of CRISPCheckerNoir clones with configuration data.
contract CRISPCheckerNoirFactory is Factory {
    /// @notice Initializes the factory with the CRISPCheckerNoir implementation.
    /// @dev The constructor sets the CRISPCheckerNoir contract as the implementation for cloning.
    constructor() Factory(address(new CRISPCheckerNoir())) {}

    /// @notice Deploys a new CRISPCheckerNoir clone with the specified Semaphore Noir contract and group ID.
    /// @dev Encodes the Semaphore Noir contract address and group ID as initialization data for the clone.
    /// @param semaphoreNoir Address of the Semaphore Noir contract.
    /// @param groupId Unique identifier of the Semaphore group.
    /// @return clone The address of the newly deployed CRISPCheckerNoir clone.
    function deploy(
        address semaphoreNoir,
        uint256 groupId
    ) public returns (address clone) {
        bytes memory data = abi.encode(semaphoreNoir, groupId);

        clone = super._deploy(data);

        CRISPCheckerNoir(clone).initialize();
    }
}