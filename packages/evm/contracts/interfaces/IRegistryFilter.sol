// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

interface IRegistryFilter {
    function requestCommittee(
        uint256 e3Id,
        uint32[2] calldata threshold
    ) external returns (bool success);
}
