// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IE3Program, IEnclavePolicy } from "../interfaces/IE3Program.sol";
import { IEnclavePolicyFactory } from "../interfaces/IEnclavePolicyFactory.sol";
import { IEnclaveChecker } from "../interfaces/IEnclaveChecker.sol";

contract MockE3Program is IE3Program {
    error invalidParams(bytes e3ProgramParams, bytes computeProviderParams);

    bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");

    IEnclavePolicyFactory private immutable policyFactory;
    address private immutable enclaveChecker;

    constructor(IEnclavePolicyFactory _policyFactory, address _enclaveChecker) {
        policyFactory = _policyFactory;
        enclaveChecker = _enclaveChecker;
    }

    function validate(
        uint256,
        uint256,
        uint8 inputLimit,
        bytes memory e3ProgramParams,
        bytes memory computeProviderParams
    )
        external
        returns (bytes32 encryptionSchemeId, IEnclavePolicy inputValidator)
    {
        require(
            computeProviderParams.length == 32,
            invalidParams(e3ProgramParams, computeProviderParams)
        );

        inputValidator = IEnclavePolicy(
            policyFactory.deploy(enclaveChecker, inputLimit)
        );
        inputValidator.setTarget(msg.sender);
        encryptionSchemeId = ENCRYPTION_SCHEME_ID;
    }

    function verify(
        uint256,
        bytes32,
        bytes memory data
    ) external pure returns (bool success) {
        data;
        if (data.length > 0) success = true;
    }
}
