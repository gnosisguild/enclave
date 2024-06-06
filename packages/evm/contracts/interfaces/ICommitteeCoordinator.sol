// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

interface ICommitteeCoordinator {
    /// @notice This event MUST be emitted when a committee is selected for an E3.
    /// @param e3Id ID of the E3 for which the committee was selected.
    /// @param threshold The M/N threshold for the committee.
    event CommitteeRequested(uint256 indexed e3Id, uint32[2] threshold);

    /// @notice This event MUST be emitted when a committee is selected for an E3.
    /// @param e3Id ID of the E3 for which the committee was selected.
    /// @param publicKey Public key of the committee.
    event CommitteeAssembled(uint256 indexed e3Id, bytes publicKey);

    /// @notice This function should be called by the Enclave contract to select a node committee.
    /// @param e3Id ID of the E3 for which to select the committee.
    /// @param threshold The M/N threshold for the committee.
    /// @return success True if committee selection was successfully initiated.
    function requestCommittee(
        uint256 e3Id,
        uint32[2] calldata threshold
    ) external returns (bool success);

    /// @notice This function should be called by the Enclave contract to get the public key of a committee.
    /// @dev This function MUST revert if no committee has been requested for the given E3.
    /// @dev This function MUST revert if the committee has not yet published a public key.
    /// @param e3Id ID of the E3 for which to get the committee public key.
    /// @return publicKey The public key of the committee.
    function committeePublicKey(
        uint256 e3Id
    ) external view returns (bytes memory);
}
