// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IComputationModule, IInputValidator } from "../interfaces/IComputationModule.sol";

contract MockComputationModule is IComputationModule {
    error invalidParams(bytes params);

    function validate(bytes memory params) external pure returns (IInputValidator inputValidator) {
        require(params.length == 32, "invalid params");
        // solhint-disable no-inline-assembly
        assembly {
            inputValidator := mload(add(params, 32))
        }
    }

    function verify(
        uint256,
        bytes memory outputData
    ) external pure returns (bytes memory output, bool success) {
        return (outputData, true);
    }
}
