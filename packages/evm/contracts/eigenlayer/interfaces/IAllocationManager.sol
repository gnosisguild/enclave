// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IStrategy } from "./IStrategy.sol";

interface IAllocationManager {
    struct OperatorSet {
        address avs;
        uint32 operatorSetId;
    }

    struct SlashingParams {
        address operator;
        uint32 operatorSetId;
        IStrategy[] strategies;
        uint256[] wadsToSlash;
        string description;
    }

    function slashOperator(
        address avs,
        SlashingParams calldata slashingParams
    ) external returns (uint256 slashingNonce);

    function getAllocatedMagnitude(
        address operator,
        OperatorSet calldata operatorSet,
        IStrategy strategy
    ) external view returns (uint256 magnitude);

    function getTotalMagnitude(
        address operator,
        IStrategy strategy
    ) external view returns (uint256 magnitude);

    event OperatorSlashed(
        address indexed operator,
        OperatorSet indexed operatorSet,
        IStrategy[] strategies,
        uint256[] wadsSlashed,
        string description
    );
}
