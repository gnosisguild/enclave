// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IExecutionModule, IOutputVerifier } from "../interfaces/IExecutionModule.sol";

contract MockExecutionModule is IExecutionModule {
    function validate(bytes calldata params) external pure returns (IOutputVerifier outputVerifier) {
        (outputVerifier) = abi.decode(params, (IOutputVerifier));
    }
}
