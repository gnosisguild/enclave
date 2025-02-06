// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IInputValidator } from "../interfaces/IInputValidator.sol";
import { IGrecoVerifier } from "../interfaces/IGrecoVerifier.sol";

contract MockInputValidator is IInputValidator {
    IGrecoVerifier public verifier;

    constructor(address _verifier) {
        verifier = IGrecoVerifier(_verifier);
    }

    function validate(
        address,
        bytes memory params
    ) external view returns (bytes memory input, bool success) {
        (
            bytes memory proof,
            uint256[] memory instances,
            bytes memory encryptedInput
        ) = abi.decode(params, (bytes, uint256[], bytes));

        success = verifier.verifyProof(proof, instances);
        if (success) {
            input = encryptedInput;
        }
    }
}
