// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {IEnclavePolicy} from "@gnosis-guild/enclave/contracts/interfaces/IEnclavePolicy.sol";
import {AdvancedPolicy} from "@excubiae/contracts/policy/AdvancedPolicy.sol";
import {AdvancedChecker} from "@excubiae/contracts/checker/AdvancedChecker.sol";
import {Check} from "@excubiae/contracts/interfaces/IAdvancedChecker.sol";
import {ISemaphore} from "@semaphore-protocol/contracts/interfaces/ISemaphore.sol";

/// @title CRISPPolicy
/// @notice Policy contract for validating inputs based on Semaphore proofs and usage limits.
contract CRISPPolicy is AdvancedPolicy, IEnclavePolicy {
    /// Errors
    error MainCalledTooManyTimes();
    error InvalidInitializationAddress();
    error AlreadyEnforced();

    /// State Variables
    uint8 public inputLimit;
    mapping(address subject => uint8 count) public enforced;
    mapping(uint256 => bool) public spentNullifiers;

    /// @notice Initializes the contract with appended bytes data for configuration.
    /// @dev Decodes AdvancedChecker address and sets the owner.
    function _initialize() internal virtual override {
        bytes memory data = _getAppendedBytes();
        (address sender, address advCheckerAddr, uint8 _inputLimit) = abi
            .decode(data, (address, address, uint8));
        _transferOwnership(sender);

        ADVANCED_CHECKER = AdvancedChecker(advCheckerAddr);
        SKIP_PRE = true;
        SKIP_POST = true;
        inputLimit = _inputLimit;
    }

    /// @notice Internal enforcement logic: checks nullifier, input limit, and marks nullifier spent.
    /// @param subject The interacting address.
    /// @param evidence Abi-encoded `ISemaphore.SemaphoreProof`.
    /// @param checkType For multi-phase policy, this is the phase to enforce.
    function _enforce(
        address subject,
        bytes calldata evidence,
        Check checkType
    ) internal override(AdvancedPolicy) onlyTarget {
        ISemaphore.SemaphoreProof memory proof = abi.decode(
            evidence,
            (ISemaphore.SemaphoreProof)
        );
        uint256 _nullifier = proof.nullifier;

        if (spentNullifiers[_nullifier]) {
            revert AlreadyEnforced();
        }
        spentNullifiers[_nullifier] = true;

        uint8 count = enforced[subject];
        if (inputLimit > 0 && count == inputLimit) {
            revert MainCalledTooManyTimes();
        }

        super._enforce(subject, evidence, checkType);
        enforced[subject]++;
    }

    /// @notice Returns policy identifier "CRISPPolicy".
    function trait() external pure returns (string memory) {
        return "CRISPPolicy";
    }
}
