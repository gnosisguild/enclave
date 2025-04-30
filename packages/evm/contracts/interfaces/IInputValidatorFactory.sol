// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IFactory } from "@excubiae/contracts/interfaces/IFactory.sol";

interface IInputValidatorFactory is IFactory {
    function deploy(address _policyAddr) external returns (address clone);
}
