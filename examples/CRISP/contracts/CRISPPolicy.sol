// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {BasePolicy} from "@excubiae/contracts/policy/BasePolicy.sol";
import {BaseChecker} from "@excubiae/contracts/checker/BaseChecker.sol";
import {ISemaphore} from "@semaphore-protocol/contracts/interfaces/ISemaphoreNoir.sol";

/// @title CRISPPolicy
/// @notice Policy contract for validating inputs based on Semaphore proofs and usage limits.
contract CRISPPolicy is BasePolicy {
    /// Errors
    error MainCalledTooManyTimes();
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

    /// @notice Internal enforcement logic: checks nullifier, input limit, and marks nullifier spent.
    /// @param subject The interacting address.
    /// @param evidence Abi-encoded `ISemaphore.SemaphoreProof`.
    function _enforce(
        address subject,
        bytes calldata evidence
    ) internal override(BasePolicy) onlyTarget {
        ISemaphore.SemaphoreNoirProof memory proof = abi.decode(
            evidence,
            (ISemaphore.SemaphoreNoirProof)
        );
        uint256 n = proof.nullifier;
        if (spentNullifiers[n]) revert AlreadyEnforced();
        if (inputLimit > 0 && enforced[subject] == inputLimit)
            revert MainCalledTooManyTimes();

        super._enforce(subject, evidence);
        spentNullifiers[n] = true;
        enforced[subject]++;
    }

    /// @notice Returns policy identifier "CRISPPolicy".
    function trait() external pure returns (string memory) {
        return "CRISPPolicy";
    }
}
