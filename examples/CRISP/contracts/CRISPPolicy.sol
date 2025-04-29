// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {IEnclavePolicy} from "@gnosis-guild/enclave/contracts/interfaces/IEnclavePolicy.sol";
import {BasePolicy} from "@excubiae/contracts/policy/BasePolicy.sol";
import {BaseChecker} from "@excubiae/contracts/checker/BaseChecker.sol";
import {ISemaphore} from "@semaphore-protocol/contracts/interfaces/ISemaphore.sol";

/// @title CRISPPolicy
/// @notice Policy contract for validating inputs based on Semaphore proofs and usage limits.
contract CRISPPolicy is BasePolicy, IEnclavePolicy {
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
        (address sender, address baseCheckerAddr, uint8 _inputLimit) = abi
            .decode(data, (address, address, uint8));
        _transferOwnership(sender);

        BASE_CHECKER = BaseChecker(baseCheckerAddr);
        inputLimit = _inputLimit;
    }

    /// @notice Validate the input and return the vote.
    /// @param subject The subject to validate the policy on.
    /// @param evidence Abi-encoded `ISemaphore.SemaphoreProof`.
    function validate(
        address subject,
        bytes calldata evidence
    ) external override onlyTarget returns (bytes memory voteBytes) {
        (bytes memory proofBytes, bytes memory vote) = abi.decode(
            evidence,
            (bytes, bytes)
        );

        _enforceChecks(subject, proofBytes);

        return vote;
    }

    /// @notice Internal enforcement logic: checks nullifier, input limit, and marks nullifier spent.
    /// @param subject The interacting address.
    /// @param evidence Abi-encoded `ISemaphore.SemaphoreProof`.
    function _enforceChecks(address subject, bytes memory evidence) internal {
        ISemaphore.SemaphoreProof memory proof = abi.decode(
            evidence,
            (ISemaphore.SemaphoreProof)
        );

        uint256 n = proof.nullifier;
        if (spentNullifiers[n]) revert AlreadyEnforced();
        spentNullifiers[n] = true;

        if (inputLimit > 0 && enforced[subject] == inputLimit)
            revert MainCalledTooManyTimes();

        if (!BASE_CHECKER.check(subject, evidence)) revert UnsuccessfulCheck();
        emit Enforced(subject, guarded, evidence);

        enforced[subject]++;
    }

    /// @notice Internal enforcement logic: checks nullifier, input limit, and marks nullifier spent.
    /// @param subject The interacting address.
    /// @param evidence Abi-encoded `ISemaphore.SemaphoreProof`.
    function _enforce(
        address subject,
        bytes calldata evidence
    ) internal override(BasePolicy) onlyTarget {
        _enforceChecks(subject, evidence);
    }

    /// @notice Returns policy identifier "CRISPPolicy".
    function trait() external pure returns (string memory) {
        return "CRISPPolicy";
    }
}
