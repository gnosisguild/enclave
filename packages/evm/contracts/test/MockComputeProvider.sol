// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {
    IComputeProvider,
    IDecryptionVerifier
} from "../interfaces/IComputeProvider.sol";

contract MockComputeProvider is IComputeProvider {
    error invalidParams();

    function validate(
        bytes memory params
    ) external pure returns (IDecryptionVerifier decryptionVerifier) {
        require(params.length == 32, invalidParams());
        // solhint-disable no-inline-assembly
        assembly {
            decryptionVerifier := mload(add(params, 32))
        }
        (decryptionVerifier) = abi.decode(params, (IDecryptionVerifier));
    }
}
