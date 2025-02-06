// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

interface IGrecoVerifier {
    function verifyProof(
        bytes calldata proof,
        uint256[] calldata instances
    ) external view returns (bool);
}
