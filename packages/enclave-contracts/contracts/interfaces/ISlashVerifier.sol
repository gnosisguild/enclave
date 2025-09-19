// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

interface ISlashVerifier {
    /// @notice This function should be called by the SlashingManager contract to verify the
    /// proof of a slash.
    /// @param proposalId ID of the proposal.
    /// @param proof ABI encoded proof of the given proposal.
    /// @return success Whether or not the proof was successfully verified.
    function verify(
        uint256 proposalId,
        bytes memory proof
    ) external view returns (bool success);
}
