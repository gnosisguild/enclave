// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {Factory} from "@excubiae/contracts/proxy/Factory.sol";
import {CRISPChecker} from "./CRISPChecker.sol";

/// @title CRISPCheckerFactory
/// @notice Factory contract for deploying minimal proxy instances of CRISPChecker.
/// @dev Utilizes the Factory pattern to streamline deployment of CRISPChecker clones with configuration data.
contract CRISPCheckerFactory is Factory {
    /// @notice Initializes the factory with the CRISPChecker implementation.
    /// @dev The constructor sets the CRISPChecker contract as the implementation for cloning.
    constructor() Factory(address(new CRISPChecker())) {}

    /// @notice Deploys a new CRISPChecker clone with the specified Semaphore contract and group ID.
    /// @dev Encodes the Semaphore contract address and group ID as initialization data for the clone.
    /// @param semaphore Address of the Semaphore contract.
    /// @param groupId Unique identifier of the Semaphore group.
    /// @return clone The address of the newly deployed CRISPChecker clone.
    function deploy(
        address semaphore,
        uint256 groupId
    ) public returns (address clone) {
        bytes memory data = abi.encode(semaphore, groupId);

        clone = super._deploy(data);

        CRISPChecker(clone).initialize();
    }
}
