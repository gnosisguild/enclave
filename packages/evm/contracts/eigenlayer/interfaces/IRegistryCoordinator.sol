// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

interface IRegistryCoordinator {
    enum OperatorStatus {
        NEVER_REGISTERED,
        REGISTERED,
        DEREGISTERED
    }

    struct OperatorInfo {
        bytes32 operatorId;
        OperatorStatus status;
    }

    function getOperatorStatus(
        address operator
    ) external view returns (OperatorStatus);

    function getOperatorId(address operator) external view returns (bytes32);
}
