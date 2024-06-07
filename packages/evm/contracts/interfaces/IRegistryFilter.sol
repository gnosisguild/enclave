// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

interface IRegistryFilter {
    event CommitteeRequested(uint256 indexed e3Id, uint32[2] threshold);

    event CommitteeCreated(
        uint256 indexed e3Id,
        bytes publicKey,
        address[] cyphernodes
    );

    function requestCommittee(
        uint256 e3Id,
        uint32[2] calldata threshold
    ) external returns (bool success);

    function retrieveCommittee(
        uint256 e3Id
    )
        external
        view
        returns (
            uint32[2] memory threshold,
            bytes memory publicKey,
            address[] memory cyphernodes
        );
}
