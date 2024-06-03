// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IExecutionModule, IOutputVerifier } from "../interfaces/IExecutionModule.sol";

contract MockExecutionModule is IExecutionModule {
    error invalidParams();

    function validate(bytes memory params) external pure returns (IOutputVerifier outputVerifier) {
        require(params.length == 32, invalidParams());
        assembly {
            outputVerifier := mload(add(params, 32))
        }
        (outputVerifier) = abi.decode(params, (IOutputVerifier));
    }
}
