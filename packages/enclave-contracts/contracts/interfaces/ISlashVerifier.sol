// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

/**
 * @title ISlashVerifier
 * @notice Interface for verifying slash proofs
 * @dev Slash verifiers implement cryptographic or logical verification of slash proposals
 */
interface ISlashVerifier {
    /// @notice Verify a slash proof
    /// @dev This function is called by the SlashingManager contract during slash proposal to verify proof validity
    /// @param proposalId ID of the slash proposal
    /// @param proof ABI encoded proof data supporting the slash
    /// @return success Whether the proof was successfully verified
    function verify(
        uint256 proposalId,
        bytes memory proof
    ) external view returns (bool success);
}
