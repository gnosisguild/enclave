// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

interface IRegistryFilter {
    struct Committee {
        address[] nodes;
        uint32[2] threshold;
        bytes32 publicKey;
    }

    function requestCommittee(
        uint256 e3Id,
        uint32[2] calldata threshold
    ) external returns (bool success);

    function getCommittee(
        uint256 e3Id
    ) external view returns (Committee memory);
}
