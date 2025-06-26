// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {BaseChecker} from "@excubiae/contracts/checker/BaseChecker.sol";
import {ISemaphore, ISemaphore as ISemaphoreNoir} from "@semaphore-protocol/contracts-noir/interfaces/ISemaphoreNoir.sol";

/// @title CRISPCheckerNoir.
/// @notice Enclave Input Validator using Semaphore Noir
/// @dev Extends BaseChecker for input verification with Noir proofs.
contract CRISPCheckerNoir is BaseChecker {
    /// @notice Address of the Semaphore Noir contract used for proof verification.
    ISemaphoreNoir public semaphoreNoir;

    /// @notice Unique identifier for the Semaphore group.
    /// @dev Proofs are validated against this specific group ID.
    uint256 public groupId;

    /// @notice custom errors
    error InvalidProver();
    error InvalidGroup();
    error InvalidProof();

    /// @notice Initializes the SemaphoreChecker with the provided Semaphore Noir contract address and group ID.
    /// @dev Decodes initialization parameters from appended bytes for clone deployments.
    function _initialize() internal override {
        super._initialize();

        bytes memory data = _getAppendedBytes();
        (address _semaphoreNoir, uint256 _groupId) = abi.decode(
            data,
            (address, uint256)
        );

        semaphoreNoir = ISemaphoreNoir(_semaphoreNoir);
        groupId = _groupId;
    }

    /// @notice Validates input using Semaphore Noir proof
    /// @param subject Address to check.
    /// @param evidence Noir proof data
    /// @return True if proof is valid
    function _check(
        address subject,
        bytes calldata evidence
    ) internal view override returns (bool) {
        super._check(subject, evidence);

        ISemaphoreNoir.SemaphoreNoirProof memory proof = abi.decode(
            evidence,
            (ISemaphoreNoir.SemaphoreNoirProof)
        );

        // The proof scope encodes both the subject address and group ID to prevent front-running attacks.
        uint256 _scope = proof.scope;

        // Extract the group ID (remaining 12 bytes, 96 bits) from the scope.
        uint96 _groupId = uint96(_scope & ((1 << 96) - 1));

        if (_groupId != groupId) {
            revert InvalidGroup();
        }

        /// Uncomment this to check the prover, this checks that the prover is the same as the subject
        /// This is not needed for the CRISP protocol, since the subject is the relayer
        /// Extract the subject's address (first 20 bytes, 160 bits) from the scope.
        // ===============================
        // address _prover = address(uint160(_scope >> 96));
        // if (_prover != subject) {
        //     revert InvalidProver();
        // }
        // ===============================

        if (!semaphoreNoir.verifyProof(groupId, proof)) {
            revert InvalidProof();
        }

        return true;
    }
}