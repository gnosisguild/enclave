// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {BaseChecker} from "@excubiae/contracts/checker/BaseChecker.sol";
import {ISemaphore} from "@semaphore-protocol/contracts/interfaces/ISemaphore.sol";

/// @title CRISPChecker.
/// @notice Enclave Input Validator
/// @dev Extends BaseChecker for input verification.
contract CRISPChecker is BaseChecker {
    /// @notice Address of the Semaphore contract used for proof verification.
    ISemaphore public semaphore;

    /// @notice Unique identifier for the Semaphore group.
    /// @dev Proofs are validated against this specific group ID.
    uint256 public groupId;

    /// @notice custom errors
    error InvalidProver();
    error InvalidGroup();
    error InvalidProof();

    /// @notice Initializes the SemaphoreChecker with the provided Semaphore contract address and group ID.
    /// @dev Decodes initialization parameters from appended bytes for clone deployments.
    function _initialize() internal override {
        super._initialize();

        bytes memory data = _getAppendedBytes();
        (address _semaphore, uint256 _groupId) = abi.decode(
            data,
            (address, uint256)
        );

        semaphore = ISemaphore(_semaphore);
        groupId = _groupId;
    }

    /// @notice Validates input
    /// @param subject Address to check.
    /// @param evidence mock proof
    /// @return True if proof is valid
    function _check(
        address subject,
        bytes calldata evidence
    ) internal view override returns (bool) {
        super._check(subject, evidence);

        ISemaphore.SemaphoreProof memory proof = abi.decode(
            evidence,
            (ISemaphore.SemaphoreProof)
        );

        // The proof scope encodes both the subject address and group ID to prevent front-running attacks.
        uint256 _scope = proof.scope;

        // Extract the subject's address (first 20 bytes, 160 bits) from the scope.
        address _prover = address(uint160(_scope >> 96));

        // Extract the group ID (remaining 12 bytes, 96 bits) from the scope.
        uint96 _groupId = uint96(_scope & ((1 << 96) - 1));

        if (_groupId != groupId) {
            revert InvalidGroup();
        }

        if (_prover != subject) {
            revert InvalidProver();
        }

        if (!semaphore.verifyProof(_scope, proof)) {
            revert InvalidProof();
        }

        return true;
    }
}
