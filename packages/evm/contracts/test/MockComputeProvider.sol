// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import {
    IComputeProvider,
    IOutputVerifier
} from "../interfaces/IComputeProvider.sol";

contract MockComputeProvider is IComputeProvider {
    error invalidParams();

    function validate(
        bytes memory params
    ) external pure returns (IOutputVerifier outputVerifier) {
        require(params.length == 32, invalidParams());
        // solhint-disable no-inline-assembly
        assembly {
            outputVerifier := mload(add(params, 32))
        }
        (outputVerifier) = abi.decode(params, (IOutputVerifier));
    }
}
