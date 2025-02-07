// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IFactory } from "@excubiae/contracts/src/core/interfaces/IFactory.sol";

interface IEnclavePolicyFactory is IFactory {
    function deploy(
        address _checkerAddr,
        uint8 _inputLimit
    ) external returns (address clone);
}
