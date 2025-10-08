// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

/**
 * @title IRegistryFilter
 * @notice Interface for filtering and selecting committee members from the registry
 * @dev Registry filters implement committee selection algorithms for E3 computations
 */
interface IRegistryFilter {
    /**
     * @notice Committee data structure
     * @param nodes Array of selected ciphernode addresses
     * @param threshold M/N threshold for the committee (M required signatures out of N members)
     * @param publicKey Hash of the committee's aggregated public key
     */
    struct Committee {
        address[] nodes;
        uint32[2] threshold;
        bytes32 publicKey;
    }

    /// @notice Request a committee for an E3 computation
    /// @dev This function is called by the CiphernodeRegistry to initiate committee selection
    /// @param e3Id ID of the E3 computation
    /// @param threshold M/N threshold for the committee
    /// @return success Whether the committee request was successful
    function requestCommittee(
        uint256 e3Id,
        uint32[2] calldata threshold
    ) external returns (bool success);

    /// @notice Get the committee for an E3 computation
    /// @dev This function returns the selected committee after it has been published
    /// @param e3Id ID of the E3 computation
    /// @return committee The selected committee data
    function getCommittee(
        uint256 e3Id
    ) external view returns (Committee memory);
}
