// SPDX-License-Identifier: MIT
//  Copyright (C) 2024 Privacy & Scaling Explorations
//  Auto-generated from https://github.com/privacy-scaling-explorations/excubiae.git@96a3312455417dc1b2e0d87066661fdf8f490fac
pragma solidity >=0.8.27;

import {IChecker} from "./interfaces/IChecker.sol";

/// @title Checker
/// @notice Abstract base contract for implementing attribute verification logic.
/// @dev Provides infrastructure to orchestrate third-party verifiers for single checks.
abstract contract Checker is IChecker {
    /// @notice Array of third-party contract addresses used for verification.
    /// @dev Can include existing and already deployed Checkers, NFTs, MACI polls, and/or any other contract
    /// that provides evidence verification. These contracts should already be deployed and operational.
    address[] internal verifiers;

    /// @notice Initializes the Checker with an optional list of third-party verification contracts.
    /// @param _verifiers Array of addresses for existing verification contracts.
    /// @dev Each address should point to a deployed contract that will be consulted during verification.
    /// This array can remain empty if there's no reliance on external verifiers.
    constructor(address[] memory _verifiers) {
        verifiers = _verifiers;
    }

    /// @notice Retrieves the verifier address at a specific index.
    /// @param index The index of the verifier in the array.
    /// @return The address of the verifier at the specified index.
    /// @custom:throws VerifierNotFound if no address have been specified at given index.
    function getVerifierAtIndex(uint256 index) external view returns (address) {
        return _getVerifierAtIndex(index);
    }

    /// @notice Internal implementation of verifier address retrieval at a specific index.
    /// @param index The index of the verifier in the array.
    /// @return The address of the verifier at the specified index.
    /// @custom:throws VerifierNotFound if no address have been specified at given index.
    function _getVerifierAtIndex(uint256 index) internal view returns (address) {
        if (index >= verifiers.length) revert VerifierNotFound();

        return verifiers[index];
    }
}
