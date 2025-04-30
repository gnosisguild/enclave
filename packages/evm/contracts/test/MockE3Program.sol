// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IE3Program, IInputValidator } from "../interfaces/IE3Program.sol";
import { IEnclavePolicy } from "../interfaces/IEnclavePolicy.sol";
import { IEnclavePolicyFactory } from "../interfaces/IEnclavePolicyFactory.sol";
import {
    IInputValidatorFactory
} from "../interfaces/IInputValidatorFactory.sol";

contract MockE3Program is IE3Program {
    error invalidParams(bytes e3ProgramParams, bytes computeProviderParams);
    error InvalidChecker();
    error InvalidPolicyFactory();
    error InvalidInputValidatorFactory();
    address private constant DO_NOT_OVERRIDE =
        0x9999999999999999999999999999999999999999;
    bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");

    IEnclavePolicyFactory private immutable POLICY_FACTORY;
    IInputValidatorFactory private immutable INPUT_VALIDATOR_FACTORY;
    address private immutable ENCLAVE_CHECKER;
    uint8 public inputLimit;

    // NOTE: this is primarily for testing
    address private overrideInputValidator = DO_NOT_OVERRIDE;

    constructor(
        IEnclavePolicyFactory _policyFactory,
        IInputValidatorFactory _inputValidatorFactory,
        address _enclaveChecker,
        uint8 _inputLimit
    ) {
        if (_enclaveChecker == address(0)) {
            revert InvalidChecker();
        }

        if (address(_policyFactory) == address(0)) {
            revert InvalidPolicyFactory();
        }

        if (address(_inputValidatorFactory) == address(0)) {
            revert InvalidInputValidatorFactory();
        }

        POLICY_FACTORY = _policyFactory;
        INPUT_VALIDATOR_FACTORY = _inputValidatorFactory;
        ENCLAVE_CHECKER = _enclaveChecker;
        inputLimit = _inputLimit;
    }

    // NOTE: This function is for testing only
    function testOverrideInputValidator(address _inputValidator) external {
        overrideInputValidator = _inputValidator;
    }

    function validate(
        uint256,
        uint256,
        bytes memory e3ProgramParams,
        bytes memory computeProviderParams
    )
        external
        returns (bytes32 encryptionSchemeId, IInputValidator inputValidator)
    {
        require(
            computeProviderParams.length == 32,
            invalidParams(e3ProgramParams, computeProviderParams)
        );

        if (overrideInputValidator == DO_NOT_OVERRIDE) {
            IEnclavePolicy policy = IEnclavePolicy(
                POLICY_FACTORY.deploy(ENCLAVE_CHECKER, inputLimit)
            );
            inputValidator = IInputValidator(
                INPUT_VALIDATOR_FACTORY.deploy(address(policy))
            );
            policy.setTarget(address(inputValidator));
        } else {
            inputValidator = IInputValidator(overrideInputValidator);
        }

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
