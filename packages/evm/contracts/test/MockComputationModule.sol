// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IComputationModule, IInputValidator } from "../interfaces/IComputationModule.sol";

contract MockComputationModule is IComputationModule {
    function validate(bytes calldata params) external pure returns (IInputValidator inputValidator) {
        (inputValidator) = abi.decode(params, (IInputValidator));
    }

    function verify(uint256, bytes memory outputData) external pure returns (bytes memory output, bool success) {
        return (outputData, true);
    }
}
